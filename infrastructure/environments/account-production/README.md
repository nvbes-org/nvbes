# Account Production Environment

Isolated production environment for the nvbes Account runtime, PostgreSQL database, and operational jobs on Scaleway.

## Architecture

- **Runtime**: Scaleway Serverless Container with `min_scale = 0`, `max_scale = 1` (scale-to-zero).
- **Database**: Scaleway Serverless SQL Database with `min_cpu = 0`, `max_cpu = 1` (scale-to-zero).
- **FinOps**: Strict <= 30 EUR/month envelope with auto-scaling to zero when idle.
- **Security**: Public signup and payments absent; RS256 token verification against Identity.
