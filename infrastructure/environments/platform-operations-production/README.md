# Platform Operations Production Infrastructure

Terraform stack for the isolated nvbes Platform Operations Cockpit on Scaleway.

## Specifications

- **Compute**: Scaleway Serverless Container (`min_scale = 0`, `max_scale = 1`, 560m CPU, 1024MB RAM).
- **Access**: Private container (`privacy = "private"`), protected behind Zero Trust and operator token auth.
- **FinOps**: Zero-idle compute cost when not accessed. Total foundation remains strictly under the 30 EUR TTC/month ceiling.
