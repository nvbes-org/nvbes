# Staging

Terraform/OpenTofu overlay for the nvbes Drive staging environment.

## Scope

- Cloudflare DNS and minimal WAF rules.
- One public API instance and one worker instance.
- Scaleway Private Network for API, worker and PostgreSQL.
- Managed PostgreSQL with private endpoint, encryption at rest and 7-day backup retention.
- Private Object Storage bucket with versioning, CORS and lifecycle rules.
- Runtime IAM application and API key for Object Storage and Secret Manager reads.
- Secret inventory to populate in Scaleway Secret Manager outside Terraform state.

## Usage

```bash
cp terraform.tfvars.example terraform.tfvars
terraform init
terraform plan
```

Do not commit `terraform.tfvars` or real secret values.
