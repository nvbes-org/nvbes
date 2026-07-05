# GraphQL Contracts

The gateway schema is a product-facing BFF contract. Resolvers must call domain
services through their owned APIs or gRPC contracts.

Rules:

- billing fields expose local IDs and provider codes, not raw PSP identifiers;
- user authorization is enforced at the gateway before service calls;
- service-to-service calls carry `RequestContext` and service credentials;
- removing or changing non-null fields requires a major schema version.
