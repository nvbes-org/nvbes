# nvbes Contracts

This directory is the source boundary for contracts consumed by active V1 runtimes.

## Layout

- `protobuf/`: internal gRPC contracts consumed by Email and Trust/Risk.
- `events/`: versioned durable event schemas.

Identity OpenAPI and generated types are owned by `libs/ts/identity-sdk-core`.
Contracts without an active consumer live under `archive/contracts`.
