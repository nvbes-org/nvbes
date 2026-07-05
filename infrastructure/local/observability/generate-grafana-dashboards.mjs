import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
const outDir = join(dirname(fileURLToPath(import.meta.url)), "grafana-dashboards");
mkdirSync(outDir, { recursive: true });
const DS = {
  prometheus: { type: "prometheus", uid: "Prometheus" },
  mimir: { type: "prometheus", uid: "Mimir" },
  loki: { type: "loki", uid: "Loki" },
  tempo: { type: "tempo", uid: "Tempo" },
  pyroscope: { type: "grafana-pyroscope-datasource", uid: "Pyroscope" },
};

const dashboards = [
  {
    file: "nvbes-platform-overview.json",
    title: "nvbes Platform Overview",
    tags: ["nvbes", "p0", "overview"],
    panels: [
      stat("Service scrape health", 'min by (job) (up{job=~"identity-api|drive-api|internal-admin|identity-worker|drive-worker"})'),
      graph("HTTP request rate by job/status", "sum by (job, status) (rate(http_requests_total[5m]))"),
      stat("5xx ratio", 'sum(rate(http_requests_total{status=~"5.."}[5m])) / clamp_min(sum(rate(http_requests_total[5m])), 1) * 100', "percent"),
      graph("HTTP latency p95/p99", "histogram_quantile(0.95, sum by (le, job) (rate(http_request_duration_seconds_bucket[5m]))) or histogram_quantile(0.99, sum by (le, job) (rate(http_request_duration_seconds_bucket[5m])))", "s"),
      stat("PostgreSQL pool saturation", "max((postgres_pool_size - postgres_pool_idle) / clamp_min(postgres_pool_size, 1) * 100)", "percent"),
      graph("Worker queue pressure", "sum by (queue, status) (worker_queue_depth)"),
      stat("Worker oldest queued job age", "max(worker_queue_oldest_age_seconds)", "s"),
      stat("Firing alerts", 'sum(ALERTS{alertstate="firing"})'),
    ],
  },
  {
    file: "nvbes-api-health.json",
    title: "nvbes API Health",
    tags: ["nvbes", "p0", "api"],
    panels: [
      graph("Request rate by service", 'sum by (job, method) (rate(http_requests_total{job=~"identity-api|drive-api|internal-admin"}[5m]))'),
      graph("Status mix", 'sum by (job, status) (rate(http_requests_total{job=~"identity-api|drive-api|internal-admin"}[5m]))'),
      graph("Latency p95 by service", 'histogram_quantile(0.95, sum by (le, job) (rate(http_request_duration_seconds_bucket{job=~"identity-api|drive-api|internal-admin"}[5m])))', "s"),
      graph("Latency p99 by service", 'histogram_quantile(0.99, sum by (le, job) (rate(http_request_duration_seconds_bucket{job=~"identity-api|drive-api|internal-admin"}[5m])))', "s"),
      stat("Active requests", "sum(http_active_requests)"),
      stat("5xx error budget burn", 'sum(rate(http_requests_total{status=~"5.."}[5m])) / clamp_min(sum(rate(http_requests_total[5m])), 1) * 100', "percent"),
    ],
  },
  {
    file: "nvbes-auth-identity-security.json",
    title: "nvbes Authentication and Identity Security",
    tags: ["nvbes", "p0", "identity", "security"],
    panels: [
      graph("OAuth token exchange", "rate(identity_oauth_token_exchange_total[5m])"),
      graph("OAuth client credentials", "rate(identity_oauth_client_credentials_total[5m])"),
      graph("OAuth policy denied", "sum by (reason) (rate(identity_oauth_policy_denied_total[5m]))"),
      graph("Auth risk and login failures", 'sum by (event) (rate(identity_auth_security_events_total{event=~"login_failed|risk_policy_blocked|lockout|step_up_failed"}[5m]))'),
      graph("MFA and WebAuthn failures", 'sum by (factor, outcome) (rate(identity_mfa_events_total{outcome!="success"}[5m]))'),
      stat("Recovery review backlog", 'max(identity_recovery_review_backlog{status="pending"})'),
    ],
  },
  {
    file: "nvbes-postgresql.json",
    title: "nvbes PostgreSQL",
    tags: ["nvbes", "p0", "postgres"],
    panels: [
      graph("Pool size and idle", "max by (service, environment) (postgres_pool_size) or max by (service, environment) (postgres_pool_idle)"),
      stat("Pool saturation", "max((postgres_pool_size - postgres_pool_idle) / clamp_min(postgres_pool_size, 1) * 100)", "percent"),
      graph("Database latency", "histogram_quantile(0.95, sum by (le, operation) (rate(postgres_operation_duration_seconds_bucket[5m])))", "s"),
      graph("Slow queries", "sum by (service) (rate(postgres_slow_queries_total[5m]))"),
      stat("Migration failures", "sum(increase(postgres_migration_failures_total[1h]))"),
      stat("Backup age", "max(postgres_backup_age_seconds)", "s"),
    ],
  },
  {
    file: "nvbes-redis-hot-path.json",
    title: "nvbes Redis and Hot Path",
    tags: ["nvbes", "p0", "redis"],
    panels: [
      graph("Redis command rate", "sum by (command, outcome) (rate(redis_commands_total[5m]))"),
      graph("Redis command p95 latency", "histogram_quantile(0.95, sum by (le, command) (rate(redis_command_duration_seconds_bucket[5m])))", "s"),
      graph("Cache hit and miss", "sum by (cache_type) (rate(redis_cache_hit_total[5m])) or sum by (cache_type) (rate(redis_cache_miss_total[5m]))"),
      graph("Rate limit decisions", "sum by (bucket, action) (rate(redis_ratelimit_allowed_total[5m])) or sum by (bucket, action) (rate(redis_ratelimit_blocked_total[5m]))"),
      stat("Redis pool active", "max(redis_pool_active)"),
      stat("Redis pool available", "max(redis_pool_available)"),
    ],
  },
  {
    file: "nvbes-workers-queues.json",
    title: "nvbes Workers and Queues",
    tags: ["nvbes", "p0", "workers"],
    panels: [
      graph("Queue depth", "sum by (queue, status) (worker_queue_depth)"),
      stat("Oldest queued job age", "max by (queue) (worker_queue_oldest_age_seconds)", "s"),
      graph("Job outcomes", "sum by (job_type, outcome) (rate(worker_queue_jobs_total[5m]))"),
      graph("Job p95 duration", "histogram_quantile(0.95, sum by (le, job_type) (rate(worker_queue_job_duration_seconds_bucket[5m])))", "s"),
      graph("Recovered jobs", "sum by (job_type, outcome) (rate(worker_queue_recovered_jobs_total[5m]))"),
      stat("Dead letters", 'sum(increase(worker_queue_jobs_total{outcome="dead_letter"}[1h]))'),
    ],
  },
  simple("nvbes-billing-stripe-webhooks.json", "nvbes Billing and Stripe Webhooks", "p0", [
    graph("Billing operations", "sum by (operation, outcome) (rate(billing_operations_total[5m]))"),
    graph("Billing operation p95", "histogram_quantile(0.95, sum by (le, operation) (rate(billing_operation_duration_seconds_bucket[5m])))", "s"),
    graph("Provider webhooks", "sum by (provider, event_type, outcome) (rate(billing_webhooks_total[5m]))"),
    graph("Webhook p95", "histogram_quantile(0.95, sum by (le, provider, event_type) (rate(billing_webhook_duration_seconds_bucket[5m])))", "s"),
    stat("Webhook failures", 'sum(increase(billing_webhooks_total{outcome=~"failure|failed"}[1h]))'),
    stat("Ledger drift", "sum(billing_ledger_drift_total)"),
  ]),
  simple("nvbes-object-storage-upload-download.json", "nvbes Object Storage and Upload Download", "p0", [
    graph("Upload outcomes", "sum by (operation, outcome) (rate(upload_operations_total[5m]))"),
    graph("Download outcomes", "sum by (operation, outcome) (rate(download_operations_total[5m]))"),
    graph("Upload p95 duration", "histogram_quantile(0.95, sum by (le, operation) (rate(upload_operation_duration_seconds_bucket[5m])))", "s"),
    graph("Download p95 duration", "histogram_quantile(0.95, sum by (le, operation) (rate(download_operation_duration_seconds_bucket[5m])))", "s"),
    graph("Object storage operations", "sum by (operation, outcome) (rate(object_storage_operations_total[5m]))"),
    graph("Object storage object count", "sum by (operation, outcome) (rate(object_storage_objects_total[5m]))"),
  ]),
  simple("nvbes-edge-public-entry.json", "nvbes Edge and Public Entry", "p0", [
    stat("Blackbox probe success", "avg(probe_success)"),
    graph("Probe duration", "histogram_quantile(0.95, sum by (le, instance) (rate(probe_http_duration_seconds_bucket[5m])))", "s"),
    graph("Edge status mix", "sum by (zone, status) (rate(cloudflare_http_requests_total[5m]))"),
    graph("WAF and rate-limit actions", "sum by (action, rule) (rate(cloudflare_security_events_total[5m]))"),
    stat("TLS certificate expiry", "min(probe_ssl_earliest_cert_expiry - time())", "s"),
    stat("DNS probe failures", "sum(increase(dns_probe_failures_total[15m]))"),
  ]),
  simple("nvbes-audit-evidence.json", "nvbes Audit Evidence", "p0", [
    graph("Audit writes", "sum by (outcome) (rate(audit_events_total[5m]))"),
    stat("Actorless events", "sum(increase(audit_actorless_events_total[24h]))"),
    stat("Missing hashes", "sum(audit_missing_hash_total)"),
    stat("Hash anomalies", "sum(audit_hash_anomaly_total)"),
    stat("Chain heads", "sum(audit_chain_head_total)"),
    graph("Signing keys", "sum by (status) (audit_signing_keys_total)"),
  ]),
  simple("nvbes-privacy-gdpr-jobs.json", "nvbes Privacy and GDPR Jobs", "p0", [
    graph("Privacy requests", "sum by (request_type, status) (privacy_requests_total)"),
    graph("Privacy job outcomes", 'sum by (job_type, outcome) (rate(worker_queue_jobs_total{job_type=~"privacy.*|email.send"}[5m]))'),
    stat("Privacy SLA breaches", "sum(privacy_request_sla_breaches_total)"),
    stat("Legal hold blockers", "sum(privacy_legal_hold_blockers_total)"),
    graph("Export file generation", "sum by (outcome) (rate(privacy_export_files_total[5m]))"),
    graph("GDPR email delivery", 'sum by (outcome) (rate(email_messages_total{business_type=~"data_export|account_delete"}[5m]))'),
  ]),
  simple("nvbes-observability-pipeline.json", "nvbes Observability Pipeline", "p0", [
    stat("Grafana stack scrape health", 'min by (job, instance) (up{job="grafana-stack"})'),
    graph("Prometheus samples ingested", "rate(prometheus_tsdb_head_samples_appended_total[5m])"),
    graph("Mimir remote write", "sum by (status_code) (rate(prometheus_remote_storage_samples_total[5m]))"),
    graph("Tempo spans", "sum(rate(tempo_distributor_spans_received_total[5m]))"),
    graph("Loki ingest", "sum(rate(loki_distributor_bytes_received_total[5m]))"),
    graph("Pyroscope writes", "sum by (status_code) (rate(pyroscope_write_sent_bytes_total[5m]))"),
  ]),
  {
    file: "nvbes-traces-profiles.json",
    title: "nvbes Traces and Profiles",
    tags: ["nvbes", "p1", "traces", "profiles"],
    panels: [
      table("Recent API traces", "{ .service.name =~ \"nvbes-.*\" }", DS.tempo, "traceql"),
      graph("HTTP latency p95", "histogram_quantile(0.95, sum by (le, job) (rate(http_request_duration_seconds_bucket[5m])))", "s"),
      graph("Trace errors by service", 'sum by (service) (rate(traces_spanmetrics_calls_total{status_code="STATUS_CODE_ERROR"}[5m]))'),
      graph("DB and Redis dependency latency", 'histogram_quantile(0.95, sum by (le, db_system) (rate(traces_spanmetrics_latency_bucket{db_system=~"postgresql|redis"}[5m])))', "s"),
      graph("Profile export bytes", "sum by (status_code) (rate(pyroscope_write_sent_bytes_total[5m]))"),
      table("CPU profiles", 'process_cpu:cpu:nanoseconds:cpu:nanoseconds{}', DS.pyroscope, "profile"),
    ],
  },
  simple("nvbes-logs-correlation.json", "nvbes Logs and Correlation", "p1", [
    log("Errors by service", 'sum by (service) (count_over_time({platform="nvbes"} | json | level=~"error|ERROR" [5m]))'),
    log("Request IDs missing", 'sum by (service) (count_over_time({platform="nvbes"} | json | request_id="" [5m]))'),
    log("Redaction drops", 'sum by (service) (count_over_time({platform="nvbes"} |= "redaction" [5m]))'),
    log("Log volume", 'sum by (environment, service) (bytes_rate({platform="nvbes"}[5m]))'),
  ]),
  simple("nvbes-frontend-rum-faro.json", "nvbes Frontend RUM and Faro", "p1", [
    graph("Web vitals p75", "histogram_quantile(0.75, sum by (le, app, metric) (rate(faro_web_vitals_duration_seconds_bucket[5m])))", "s"),
    graph("Frontend errors", "sum by (app, route) (rate(faro_frontend_errors_total[5m]))"),
    graph("Route load p95", "histogram_quantile(0.95, sum by (le, app, route) (rate(faro_route_load_duration_seconds_bucket[5m])))", "s"),
    graph("Browser API latency", "histogram_quantile(0.95, sum by (le, app, target_service) (rate(faro_api_request_duration_seconds_bucket[5m])))", "s"),
    stat("Consented sessions", "sum(faro_consented_sessions_total)"),
  ]),
  simple("nvbes-security-abuse.json", "nvbes Security and Abuse", "p1", [
    graph("Drive authz denied", "sum by (reason) (rate(drive_authz_denied_total[5m]))"),
    graph("Public API geo requests", "sum by (status, network_kind, risk_bucket) (rate(drive_public_api_geo_requests_total[5m]))"),
    graph("High risk public API requests", "sum by (network_kind) (rate(drive_public_api_high_risk_requests_total[5m]))"),
    graph("Network policy blocks", "sum by (reason, mode) (rate(drive_public_api_network_policy_blocks_total[5m]))"),
    graph("Internal admin denied", "sum by (reason) (rate(internal_admin_permission_denied_total[5m]))"),
  ]),
  simple("nvbes-public-api-developer-platform.json", "nvbes Public API and Developer Platform", "p1", [
    graph("OAuth machine flows", "rate(identity_oauth_client_credentials_total[5m]) or rate(identity_oauth_token_exchange_total[5m])"),
    graph("Legacy API key requests", "rate(drive_api_key_legacy_requests_total[5m])"),
    graph("Identity introspection", "rate(drive_identity_introspection_total[5m]) or rate(drive_identity_introspection_cached_total[5m])"),
    graph("Identity introspection p95", "histogram_quantile(0.95, sum by (le) (rate(drive_identity_introspection_duration_seconds_bucket[5m])))", "s"),
    graph("Webhook delivery failures", "sum by (endpoint) (rate(developer_webhook_delivery_failures_total[5m]))"),
  ]),
  simple("nvbes-email-communications.json", "nvbes Email and Communications", "p1", [
    graph("Email message outcomes", "sum by (business_type, outcome) (rate(email_messages_total[5m]))"),
    graph("Provider delivery latency", "histogram_quantile(0.95, sum by (le, provider) (rate(email_delivery_duration_seconds_bucket[5m])))", "s"),
    stat("Queued emails", 'sum(worker_queue_depth{queue="email.send"})'),
    stat("Suppressed emails", "sum(email_suppressed_total)"),
    graph("Provider webhook outcomes", "sum by (provider, outcome) (rate(email_provider_webhooks_total[5m]))"),
  ]),
  simple("nvbes-internal-admin-operator-safety.json", "nvbes Internal Admin Operator Safety", "p1", [
    graph("Admin action requests", "sum by (policy, outcome) (rate(internal_admin_action_requests_total[5m]))"),
    graph("Admin action p95", "histogram_quantile(0.95, sum by (le, policy) (rate(internal_admin_action_request_duration_seconds_bucket[5m])))", "s"),
    graph("Guard rejections", "sum by (guard, reason) (rate(internal_admin_guard_rejections_total[5m]))"),
    graph("Permission denied", "sum by (reason) (rate(internal_admin_permission_denied_total[5m]))"),
    graph("Rate limited", "sum by (policy, partition) (rate(internal_admin_rate_limited_total[5m]))"),
  ]),
  simple("nvbes-synthetic-smoke-load-tests.json", "nvbes Synthetic Smoke and Load Tests", "p1", [
    graph("k6 request rate", "sum by (scenario, status) (rate(k6_http_reqs_total[5m]))"),
    graph("k6 p95 duration", "histogram_quantile(0.95, sum by (le, scenario) (rate(k6_http_req_duration_seconds_bucket[5m])))", "s"),
    stat("k6 checks passing", "avg(k6_checks_rate)", "percentunit"),
    stat("Blackbox success", "avg by (instance) (probe_success)"),
    graph("Production smoke latency", "histogram_quantile(0.95, sum by (le, journey) (rate(synthetic_journey_duration_seconds_bucket[5m])))", "s"),
  ]),
  simple("nvbes-backup-restore.json", "nvbes Backup and Restore", "p1", [
    stat("PostgreSQL backup age", "max(postgres_backup_age_seconds)", "s"),
    stat("Object storage backup age", "max(object_storage_backup_age_seconds)", "s"),
    stat("Backup failures", "sum(increase(backup_failures_total[24h]))"),
    stat("Last restore test age", "max(restore_test_age_seconds)", "s"),
    graph("Restore test duration", "histogram_quantile(0.95, sum by (le, target) (rate(restore_test_duration_seconds_bucket[30d])))", "s"),
  ]),
  simple("nvbes-finops.json", "nvbes FinOps", "p2", [
    graph("Cloud cost by environment", "sum by (environment, service) (cloud_cost_amount_eur)"),
    graph("Storage cost by workspace", "topk(20, sum by (workspace_id) (storage_cost_amount_eur))"),
    graph("Egress by workspace", "topk(20, sum by (workspace_id) (egress_bytes_total))"),
    stat("Budget used", "max by (environment) (cloud_budget_used_ratio)", "percentunit"),
    graph("Gross margin by plan", "sum by (plan_code) (billing_gross_margin_ratio)", "percentunit"),
  ]),
  simple("nvbes-product-analytics.json", "nvbes Product Analytics", "p2", [
    graph("Activation funnel", "sum by (event_name) (increase(product_events_total{event_name=~\"signup|workspace_created|first_upload|first_share\"}[24h]))"),
    graph("Core Drive usage", 'sum by (event_name) (increase(product_events_total{event_name=~"file_uploaded|file_downloaded|share_created"}[24h]))'),
    graph("Retention cohorts", "sum by (cohort) (product_retained_workspaces_total)"),
    graph("Conversion by plan", "sum by (plan_code) (increase(product_conversion_events_total[24h]))"),
  ]),
  simple("nvbes-region-data-residency.json", "nvbes Region and Data Residency", "p2", [
    graph("Geo resolutions", "sum by (outcome) (rate(geo_resolutions_total[5m]))"),
    graph("Geo cache latency", "histogram_quantile(0.95, sum by (le, provider) (rate(geo_cache_lookup_duration_seconds_bucket[5m])))", "s"),
    graph("RDAP and IP intelligence", "rate(geo_rdap_lookups_total[5m]) or rate(geo_ip_intelligence_lookups_total[5m])"),
    stat("Residency anomalies", "sum(data_residency_anomalies_total)"),
    graph("Workspace regions", "sum by (region, jurisdiction) (workspace_region_total)"),
  ]),
  simple("nvbes-cicd-release-health.json", "nvbes CI CD and Release Health", "p2", [
    graph("Deployment outcomes", "sum by (environment, service, outcome) (increase(deployments_total[24h]))"),
    stat("Failed migrations", "sum(increase(postgres_migration_failures_total[24h]))"),
    graph("Post-release smoke", "sum by (environment, journey, outcome) (increase(release_smoke_tests_total[24h]))"),
    stat("Rollback signals", "sum(increase(release_rollbacks_total[7d]))"),
    graph("Release error rate", "sum by (release, service) (rate(http_requests_total{status=~\"5..\"}[5m]))"),
  ]),
];

for (const d of dashboards) writeFileSync(join(outDir, d.file), `${JSON.stringify(dashboard(d))}\n`);

function dashboard(d) {
  return {
    annotations: { list: [{ builtIn: 1, datasource: { type: "grafana", uid: "-- Grafana --" }, enable: true, hide: true, iconColor: "rgba(0, 211, 255, 1)", name: "Annotations & Alerts", type: "dashboard" }] },
    editable: true,
    fiscalYearStartMonth: 0,
    graphTooltip: 0,
    id: null,
    links: [],
    liveNow: false,
    panels: d.panels.map((p, i) => panel(p, i)),
    refresh: "30s",
    schemaVersion: 39,
    style: "dark",
    tags: d.tags,
    templating: { list: [] },
    time: { from: "now-6h", to: "now" },
    timepicker: {},
    timezone: "browser",
    title: d.title,
    uid: d.file.replace(".json", ""),
    version: 1,
    weekStart: "",
  };
}

function panel(p, i) {
  const w = 12;
  const h = p.kind === "table" || p.kind === "logs" ? 8 : 7;
  const gridPos = { h, w, x: (i % 2) * w, y: Math.floor(i / 2) * h };
  const base = { datasource: p.datasource, description: p.description ?? "", fieldConfig: { defaults: { color: { mode: "palette-classic" }, mappings: [], thresholds: { mode: "absolute", steps: [{ color: "green", value: null }] }, unit: p.unit ?? "short" }, overrides: [] }, gridPos, id: i + 1, options: {}, targets: [target(p)], title: p.title, type: p.type };
  if (p.type === "timeseries") base.options = { legend: { displayMode: "table", placement: "bottom", showLegend: true }, tooltip: { mode: "multi", sort: "none" } };
  if (p.type === "stat") base.options = { colorMode: "value", graphMode: "area", justifyMode: "auto", orientation: "auto", reduceOptions: { calcs: ["lastNotNull"], fields: "", values: false }, textMode: "auto" };
  if (p.type === "table") base.options = { showHeader: true };
  if (p.type === "logs") base.options = { dedupStrategy: "none", enableLogDetails: true, showCommonLabels: false, showLabels: false, showTime: true, sortOrder: "Descending", wrapLogMessage: false };
  return base;
}

function target(p) {
  if (p.kind === "logs") return { datasource: p.datasource, editorMode: "code", expr: p.expr, queryType: "range", refId: "A" };
  if (p.kind === "traceql") return { datasource: p.datasource, filters: [], limit: 20, query: p.expr, queryType: "traceqlSearch", refId: "A", tableType: "traces" };
  if (p.kind === "profile") return { datasource: p.datasource, groupBy: [], labelSelector: p.expr, profileTypeId: "process_cpu:cpu:nanoseconds:cpu:nanoseconds", queryType: "profile", refId: "A" };
  return { datasource: p.datasource, editorMode: "code", expr: p.expr, instant: p.type === "stat", legendFormat: "__auto", range: p.type !== "stat", refId: "A" };
}

function graph(title, expr, unit) {
  return { title, expr, unit, type: "timeseries", datasource: DS.prometheus };
}
function stat(title, expr, unit) {
  return { title, expr, unit, type: "stat", datasource: DS.prometheus };
}
function table(title, expr, datasource, kind = "prometheus") {
  return { title, expr, type: "table", datasource, kind };
}
function log(title, expr) {
  return { title, expr, type: "logs", datasource: DS.loki, kind: "logs" };
}
function simple(file, title, priority, panels) {
  return { file, title, tags: ["nvbes", priority], panels };
}
