# Active Protobuf Contracts

Email and Trust/Risk use these protobuf contracts for active service boundaries.

Rules:

- proto files use `syntax = "proto3"`;
- packages are versioned under `nvbes.<domain>.vN`;
- service calls must carry request context and deadlines in implementation;
- compatibility is checked in CI before migration gates pass.
