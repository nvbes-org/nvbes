# Protobuf Contracts

Internal APIs use protobuf contracts for service-to-service boundaries.

Rules:

- proto files use `syntax = "proto3"`;
- packages are versioned under `nvbes.<domain>.vN`;
- service calls must carry request context and deadlines in implementation;
- compatibility is checked in CI before migration gates pass.
