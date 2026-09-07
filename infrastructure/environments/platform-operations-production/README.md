# Platform Operations Production Infrastructure

Terraform stack for the isolated nvbes Platform Operations Cockpit on Scaleway.

## Specifications

- **Compute**: Scaleway Serverless Container (`min_scale = 0`, `max_scale = 1`, 560m CPU, 1024MB RAM).
- **Access**: Private container (`privacy = "private"`), protected behind Zero Trust and signed operator JWTs with MFA and `platform_owner` role.
- **Storage**: Dedicated Operations database and restricted runtime role on the existing PostgreSQL instance. Apply migrations with a separate owner before activation. No domain database access.
- **Configuration**: Set `platform_operations_database_url`, `platform_operations_public_key_pem`, `platform_operations_issuer` and optionally `platform_operations_services`. Static operator tokens are no longer accepted.
- **FinOps**: Zero-idle compute cost when not accessed. Total foundation remains strictly under the 30 EUR TTC/month ceiling.
