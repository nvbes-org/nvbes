# Active Protobuf Contracts

Email and Trust/Risk use these protobuf contracts for active service boundaries.

Rules:

- proto files use `syntax = "proto3"`;
- `buf.yaml` applies the `STANDARD` lint rules except the three RPC naming rules
  that conflict with the existing V1 domain-type contracts;
- CI rejects `FILE`-level breaking changes against its exact planned base SHA;
- packages are versioned under `nvbes.<domain>.vN`;
- service calls must carry request context and deadlines in implementation;
- compatibility is checked in CI before migration gates pass.
