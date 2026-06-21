# nvbes Contracts

This directory is the source boundary for public REST contracts, internal
protobuf contracts and durable event schemas.

## Layout

- `openapi/`: manifest for REST OpenAPI documents.
- `protobuf/`: internal gRPC protobuf contracts.
- `events/`: versioned durable event schemas.

Generated SDKs and service code must derive from these contracts or from the
OpenAPI files referenced by the manifest.
