#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

import { checkBillingRpcRuntimeScope } from './checks.billing-rpc-scope.mjs';
import { checkOpenApiTaxonomy } from './checks.openapi-taxonomy.mjs';
import { checkSplitProtoContracts, requiredSplitEventTypes } from './checks.split-contracts.mjs';

const errors = [];

function readJson(path) {
  if (!existsSync(path)) {
    errors.push(`${path}: missing`);
    return undefined;
  }

  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    errors.push(`${path}: invalid JSON: ${error.message}`);
    return undefined;
  }
}

function walk(dir, predicate, results = []) {
  if (!existsSync(dir)) return results;
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      walk(path, predicate, results);
    } else if (predicate(path)) {
      results.push(path);
    }
  }
  return results;
}

function routePathsFromSource(dir, api) {
  const files = walk(dir, (path) => path.endsWith('.rs'));
  const paths = new Map();

  for (const file of files) {
    const content = readFileSync(file, 'utf8');
    for (const match of content.matchAll(/path\s*=\s*"([^"]+)"/g)) {
      const path = match[1];
      if (!path.startsWith('/')) continue;
      if (api.versionPrefix && !path.startsWith(api.versionPrefix)) continue;
      if (!paths.has(path)) paths.set(path, []);
      paths.get(path).push(file);
    }
  }

  return paths;
}

function checkOpenApi() {
  const manifest = readJson('contracts/openapi/manifest.json');
  if (!manifest) return;
  if (!Array.isArray(manifest.apis) || manifest.apis.length === 0) {
    errors.push('contracts/openapi/manifest.json: apis must be a non-empty array');
    return;
  }

  for (const api of manifest.apis) {
    if (!api.name || !api.document || !api.surface) {
      errors.push('contracts/openapi/manifest.json: every api needs name, surface and document');
      continue;
    }

    const spec = readJson(api.document);
    if (!spec) continue;
    if (typeof spec.openapi !== 'string' || !spec.openapi.startsWith('3.')) {
      errors.push(`${api.document}: OpenAPI 3.x document expected`);
    }
    if (!spec.info?.title || !spec.info?.version) {
      errors.push(`${api.document}: info.title and info.version are required`);
    }
    const paths = Object.keys(spec.paths ?? {});
    if (paths.length === 0) {
      errors.push(`${api.document}: at least one path is required`);
    }
    if ((api.surface === 'public' || api.surface === 'cloud-public') && api.versionPrefix) {
      if (!paths.some((path) => path.startsWith(api.versionPrefix))) {
        errors.push(`${api.document}: public API must expose ${api.versionPrefix} paths`);
      }
    }
    if (api.routeSource) {
      const sourcePaths = routePathsFromSource(api.routeSource, api);
      const specPaths = new Set(paths);
      for (const [path, files] of sourcePaths.entries()) {
        if (!specPaths.has(path)) {
          errors.push(`${api.document}: route ${path} from ${files[0]} is missing from OpenAPI`);
        }
      }
    }
  }

  checkOpenApiTaxonomy({ errors, manifest, readJson });
}

function checkProto() {
  const files = walk('contracts/protobuf', (path) => path.endsWith('.proto'));
  if (files.length === 0) {
    errors.push('contracts/protobuf: at least one .proto file is required');
  }

  for (const file of files) {
    const content = readFileSync(file, 'utf8');
    if (!content.includes('syntax = "proto3";')) {
      errors.push(`${file}: proto3 syntax is required`);
    }
    if (!/^package\s+nvbes\.[a-z0-9_.]+\.v\d+;/m.test(content)) {
      errors.push(`${file}: package must be versioned under nvbes.*.vN`);
    }
    if (/\bTODO\b|\bTBD\b|\bFIXME\b/i.test(content)) {
      errors.push(`${file}: unresolved marker`);
    }
  }
}

function checkBillingGrpcImplementation() {
  const protoPath = 'contracts/protobuf/nvbes/billing/v1/billing.proto';
  if (!existsSync(protoPath)) return;
  const proto = readFileSync(protoPath, 'utf8');
  if (!proto.includes('service BillingService')) return;

  const requiredFiles = [
    'apps/billing-service/build.rs',
    'apps/billing-service/Cargo.toml',
    'apps/billing-service/src/billing.grpc.pb.rs',
    'apps/billing-service/src/billing.grpc.service.rs',
    'apps/billing-service/src/main.rs',
  ];
  for (const file of requiredFiles) {
    if (!existsSync(file)) errors.push(`${file}: required for BillingService gRPC transport`);
  }
  if (errors.length > 0) return;

  const build = readFileSync('apps/billing-service/build.rs', 'utf8');
  if (!build.includes('tonic_prost_build::configure()')) {
    errors.push('apps/billing-service/build.rs: BillingService must be generated with tonic/prost');
  }
  if (!build.includes('contracts/protobuf/nvbes/billing/v1/billing.proto')) {
    errors.push('apps/billing-service/build.rs: BillingService proto source must be generated');
  }

  const manifest = readFileSync('apps/billing-service/Cargo.toml', 'utf8');
  for (const dependency of ['tonic', 'tonic-prost', 'tonic-prost-build']) {
    if (!manifest.includes(dependency)) {
      errors.push(`apps/billing-service/Cargo.toml: missing ${dependency} for BillingService gRPC`);
    }
  }

  const pb = readFileSync('apps/billing-service/src/billing.grpc.pb.rs', 'utf8');
  if (!pb.includes('tonic::include_proto!("nvbes.billing.v1")')) {
    errors.push('apps/billing-service/src/billing.grpc.pb.rs: missing nvbes.billing.v1 include');
  }

  const service = [
    'apps/billing-service/src/billing.grpc.service.rs',
    'apps/billing-service/src/billing.grpc.service.workspace.rs',
  ]
    .map((path) => readFileSync(path, 'utf8'))
    .join('\n');
  checkBillingRpcRuntimeScope({
    errors,
    gatewaySource: readRustSource('apps/gateway-cloud/src'),
    proto,
    protoPath,
    service,
  });

  for (const expected of [
    'ADMIN_BILLING_ACTION_KIND_CREATE_CREDIT_NOTE',
    'ADMIN_BILLING_ACTION_KIND_CREATE_WRITE_OFF',
    'ADMIN_BILLING_ACTION_KIND_CREATE_REFUND_INTENT',
    'ADMIN_BILLING_ACTION_KIND_CREATE_MANUAL_COMPENSATION',
    'ADMIN_BILLING_ACTION_KIND_REPLAY_PROVIDER_EVENT',
    'ADMIN_BILLING_ACTION_KIND_CREATE_PROVIDER_MIGRATION',
    'ADMIN_BILLING_ACTION_KIND_OVERRIDE_GRACE_PERIOD',
    'ADMIN_BILLING_PLATFORM_ACTION_KIND_APPROVE_FRAUD_ASSESSMENT',
    'ADMIN_BILLING_PLATFORM_ACTION_KIND_REJECT_FRAUD_ASSESSMENT',
    'ADMIN_BILLING_PLATFORM_ACTION_KIND_TRUST_FRAUD_ASSESSMENT',
    'ADMIN_REVENUE_ACTION_KIND_CLOSE_DUNNING_CASE',
    'ADMIN_REVENUE_ACTION_KIND_REOPEN_DUNNING_CASE',
    'ADMIN_REVENUE_ACTION_KIND_HOLD_INVOICE',
    'ADMIN_REVENUE_ACTION_KIND_RELEASE_INVOICE',
    'ADMIN_REVENUE_ACTION_KIND_REVIEW_DISPUTE',
    'ADMIN_REVENUE_ACTION_KIND_RESOLVE_DISPUTE',
  ]) {
    if (!proto.includes(expected)) {
      errors.push(`${protoPath}: missing ${expected}`);
    }
  }

  const main = readFileSync('apps/billing-service/src/main.rs', 'utf8');
  if (!main.includes('NVBES_BILLING_SERVICE_PORT')) {
    errors.push(
      'apps/billing-service/src/main.rs: Billing service must use a dedicated port env var',
    );
  }
}

function checkBillingPublicWorkspaceRoutes() {
  const billingRoutes = 'apps/billing-service/src/billing.domains.public_workspace.rs';
  const billingPortalLists =
    'apps/billing-service/src/billing.domains.public_workspace.portal_lists.rs';
  if (!existsSync(billingRoutes)) {
    errors.push(`${billingRoutes}: required for public Billing workspace routes`);
    return;
  }
  if (!existsSync(billingPortalLists)) {
    errors.push(`${billingPortalLists}: required for public Billing portal list routes`);
    return;
  }

  const content = readFileSync(billingRoutes, 'utf8');
  const surface = `${content}\n${readFileSync(billingPortalLists, 'utf8')}`;
  for (const route of [
    '"/billing/portal/capabilities"',
    '"/workspaces/{workspaceId}/billing/overview"',
    '"/workspaces/{workspaceId}/billing/usage"',
    '"/workspaces/{workspaceId}/billing/entitlements"',
    '"/workspaces/{workspaceId}/billing/checkout"',
    '"/workspaces/{workspaceId}/billing/portal"',
    '"/workspaces/{workspaceId}/billing/portal/view"',
    '"/workspaces/{workspaceId}/billing/portal/invoices/{invoiceId}/pdf"',
  ]) {
    if (!content.includes(route)) {
      errors.push(`${billingRoutes}: missing public Billing route ${route}`);
    }
  }

  for (const expected of [
    'fetch_workspace_billing_overview',
    'fetch_workspace_billing_usage',
    'fetch_workspace_entitlements',
    'create_billing_checkout_session',
    'create_billing_portal_session',
    'fetch_portal_view',
    'fetch_portal_invoices',
    'fetch_portal_payment_methods',
    'fetch_portal_subscription_providers',
    'fetch_invoice_pdf',
    'BillingWorkspacePermission::Read',
    'BillingWorkspacePermission::Manage',
  ]) {
    if (!surface.includes(expected)) {
      errors.push(`apps/billing-service/src: missing public Billing route evidence ${expected}`);
    }
  }
}

function checkGraphql() {
  const schemaPath = 'contracts/graphql/schema.graphql';
  const governancePath = 'contracts/graphql/governance.json';
  if (!existsSync(schemaPath)) {
    errors.push(`${schemaPath}: missing`);
    return;
  }
  const schema = readFileSync(schemaPath, 'utf8');
  if (!schema.includes('schema {') || !schema.includes('type Query')) {
    errors.push(`${schemaPath}: executable schema and Query type are required`);
  }
  if (/\bTODO\b|\bTBD\b|\bFIXME\b/i.test(schema)) {
    errors.push(`${schemaPath}: unresolved marker`);
  }
  for (const forbidden of [
    'checkoutSessionId',
    'portalSessionId',
    'providerCustomerId',
    'providerPaymentMethodId',
    'providerInvoiceId',
    'providerSubscriptionId',
  ]) {
    if (schema.includes(forbidden)) {
      errors.push(`${schemaPath}: ${forbidden} must not be exposed`);
    }
  }

  const governance = readJson(governancePath);
  if (!governance) return;
  if (governance.schema !== schemaPath) {
    errors.push(`${governancePath}: schema must point at ${schemaPath}`);
  }
  if (!Array.isArray(governance.owners) || governance.owners.length === 0) {
    errors.push(`${governancePath}: owners must be a non-empty array`);
  }
  if (!Array.isArray(governance.rules) || governance.rules.length === 0) {
    errors.push(`${governancePath}: rules must be a non-empty array`);
  }

  checkGraphqlGatewayImplementation(governance);
}

function checkGraphqlGatewayImplementation(governance) {
  if (!governance.entrypoints?.includes('gateway-cloud')) {
    errors.push('contracts/graphql/governance.json: gateway-cloud entrypoint is required');
  }

  const requiredFiles = [
    'apps/gateway-cloud/Cargo.toml',
    'apps/gateway-cloud/build.rs',
    'apps/gateway-cloud/src/gateway.pb.rs',
    'apps/gateway-cloud/src/gateway.billing_client.rs',
    'apps/gateway-cloud/src/gateway.auth.rs',
    'apps/gateway-cloud/src/gateway.schema.rs',
    'apps/gateway-cloud/src/gateway.schema.types.rs',
    'apps/gateway-cloud/src/main.rs',
  ];
  for (const file of requiredFiles) {
    if (!existsSync(file)) errors.push(`${file}: required for GraphQL gateway`);
  }
  if (requiredFiles.some((file) => !existsSync(file))) return;

  const manifest = readFileSync('apps/gateway-cloud/Cargo.toml', 'utf8');
  for (const dependency of ['async-graphql', 'async-graphql-axum', 'tonic', 'tonic-prost-build']) {
    if (!manifest.includes(dependency)) {
      errors.push(`apps/gateway-cloud/Cargo.toml: missing ${dependency}`);
    }
  }

  const build = readFileSync('apps/gateway-cloud/build.rs', 'utf8');
  if (!build.includes('contracts/protobuf/nvbes/billing/v1/billing.proto')) {
    errors.push('apps/gateway-cloud/build.rs: must generate BillingService proto');
  }

  const billingClient = readFileSync('apps/gateway-cloud/src/gateway.billing_client.rs', 'utf8');
  if (!billingClient.includes('BillingServiceClient::connect')) {
    errors.push(
      'apps/gateway-cloud/src/gateway.billing_client.rs: Billing gateway must use the generated gRPC client',
    );
  }
  if (
    !governance.rules.some((rule) => /deadline|timeout/i.test(`${rule.id} ${rule.description}`))
  ) {
    errors.push(
      'contracts/graphql/governance.json: Gateway gRPC deadline/timeout policy rule is required',
    );
  }

  const auth = readFileSync('apps/gateway-cloud/src/gateway.auth.rs', 'utf8');
  for (const expected of [
    'introspect_identity_token',
    'bearer_token',
    'IntrospectAccessTokenResponse',
    'principal_type',
    'network_valid',
  ]) {
    if (!auth.includes(expected)) {
      errors.push(
        `apps/gateway-cloud/src/gateway.auth.rs: Identity token introspection evidence ${expected} is required`,
      );
    }
  }
  for (const forbidden of ['x-nvbes-actor-principal-id', 'x-nvbes-tenant-id']) {
    if (auth.includes(forbidden)) {
      errors.push(
        `apps/gateway-cloud/src/gateway.auth.rs: must not trust caller-supplied identity header ${forbidden}`,
      );
    }
  }
  const mainSource = readFileSync('apps/gateway-cloud/src/main.rs', 'utf8');
  for (const expected of [
    'NVBES_IDENTITY_GRPC_ENDPOINT',
    'NVBES_GATEWAY_IDENTITY_CLIENT_ID',
    'NVBES_GATEWAY_IDENTITY_CLIENT_SECRET',
  ]) {
    if (!mainSource.includes(expected)) {
      errors.push(
        `apps/gateway-cloud/src/main.rs: Identity introspection service credential ${expected} is required`,
      );
    }
  }

  const schemaSource = readFileSync('apps/gateway-cloud/src/gateway.schema.rs', 'utf8');
  for (const expected of [
    'for_workspace(&workspace_id)',
    'for_workspace(&input.workspace_id)',
    'context: Some(request_context)',
    'create_portal',
    'CreatePortalRequest',
  ]) {
    if (!schemaSource.includes(expected)) {
      errors.push(
        `apps/gateway-cloud/src/gateway.schema.rs: Billing resolvers must propagate ${expected} to gRPC`,
      );
    }
  }
  const schemaTypes = readFileSync('apps/gateway-cloud/src/gateway.schema.types.rs', 'utf8');
  if (!schemaTypes.includes('CheckoutSession') || !schemaTypes.includes('PortalSession')) {
    errors.push(
      'apps/gateway-cloud/src/gateway.schema.types.rs: Billing session GraphQL types are required',
    );
  }

  const source = readRustSource('apps/gateway-cloud/src');
  for (const expected of [
    'get_billing_overview',
    'get_billing_portal',
    'create_checkout',
    'create_billing_portal_session',
    'GatewayRequestContext',
  ]) {
    if (!source.includes(expected)) {
      errors.push(`apps/gateway-cloud: missing GraphQL gateway resolver evidence ${expected}`);
    }
  }
  const billingGatewaySource = [billingClient, schemaSource, schemaTypes].join('\n');
  for (const forbidden of [
    'reqwest::',
    'hyper::Client',
    'billing_api_base_url',
    '/workspaces/{workspaceId}/billing',
  ]) {
    if (billingGatewaySource.includes(forbidden)) {
      errors.push(`apps/gateway-cloud: Billing gateway must use gRPC only, found ${forbidden}`);
    }
  }
  const graphqlSchema = readFileSync('contracts/graphql/schema.graphql', 'utf8');
  for (const forbidden of [
    'provider_session_id',
    'provider_customer_id',
    'provider_payment_method_id',
    'provider_invoice_id',
    'provider_subscription_id',
    'provider_event_id',
    'provider_product_id',
    'provider_price_id',
    'provider_mandate_id',
    'provider_attempt_id',
    'stripe_customer_id',
    'stripe_subscription_id',
    'stripe_payment_method_id',
    'stripe_invoice_id',
    'stripe_price_id',
    'stripe_product_id',
    'mollie_customer_id',
    'mollie_mandate_id',
    'mollie_payment_id',
    'mollie_subscription_id',
    'cb_card_id',
    'cb_token_id',
  ]) {
    if (source.includes(forbidden)) {
      errors.push(`apps/gateway-cloud: must not expose raw PSP identifier ${forbidden}`);
    }
    if (graphqlSchema.includes(forbidden)) {
      errors.push(
        `contracts/graphql/schema.graphql: must not expose raw PSP identifier ${forbidden}`,
      );
    }
  }
}

function readRustSource(dir) {
  return walk(dir, (path) => path.endsWith('.rs'))
    .map((file) => readFileSync(file, 'utf8'))
    .join('\n');
}

function checkEvents() {
  const manifest = readJson('contracts/events/manifest.json');
  const envelope = readJson('contracts/events/envelope.schema.json');
  if (!manifest || !envelope) return;

  const requiredEnvelopeFields = [
    'event_id',
    'event_type',
    'event_version',
    'tenant_id',
    'region_id',
    'occurred_at',
    'correlation_id',
    'idempotency_key',
    'payload',
  ];
  for (const field of requiredEnvelopeFields) {
    if (!envelope.required?.includes(field)) {
      errors.push(`contracts/events/envelope.schema.json: missing required field ${field}`);
    }
  }

  if (!Array.isArray(manifest.events) || manifest.events.length === 0) {
    errors.push('contracts/events/manifest.json: events must be a non-empty array');
    return;
  }

  const eventTypes = new Set(manifest.events.map((event) => event.event_type));
  for (const eventType of requiredSplitEventTypes) {
    if (!eventTypes.has(eventType)) {
      errors.push(`contracts/events/manifest.json: missing ${eventType}`);
    }
  }

  const seen = new Set();
  for (const event of manifest.events) {
    const key = `${event.event_type}@${event.event_version}`;
    if (seen.has(key)) {
      errors.push(`contracts/events/manifest.json: duplicate event ${key}`);
    }
    seen.add(key);

    if (!event.critical) {
      errors.push(
        `contracts/events/manifest.json: ${key} must declare critical=true or move out of this manifest`,
      );
    }

    const schema = readJson(event.schema);
    if (!schema) continue;
    if (schema.properties?.event_type?.const !== event.event_type) {
      errors.push(`${event.schema}: event_type const must match manifest`);
    }
    if (schema.properties?.event_version?.const !== event.event_version) {
      errors.push(`${event.schema}: event_version const must match manifest`);
    }
    if (!schema.properties?.payload || !schema.required?.includes('payload')) {
      errors.push(`${event.schema}: payload property is required`);
    }
    if (requiredSplitEventTypes.includes(event.event_type)) {
      if (schema.additionalProperties !== false) {
        errors.push(
          `${event.schema}: split event schema must set top-level additionalProperties=false`,
        );
      }
      if (schema.properties.payload.additionalProperties !== false) {
        errors.push(
          `${event.schema}: split event payload schema must set additionalProperties=false`,
        );
      }
    }
    const providerEnum = schema.properties?.payload?.properties?.provider?.enum;
    if (event.event_type.startsWith('billing.') && providerEnum) {
      for (const provider of ['stripe', 'mollie', 'cb']) {
        if (!providerEnum.includes(provider)) {
          errors.push(`${event.schema}: billing provider enum must include ${provider}`);
        }
      }
    }
  }
}

checkOpenApi();
checkProto();
checkSplitProtoContracts(errors);
checkBillingGrpcImplementation();
checkBillingPublicWorkspaceRoutes();
checkGraphql();
checkEvents();

if (errors.length > 0) {
  console.error('Contract checks failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log('Contracts: ok');
