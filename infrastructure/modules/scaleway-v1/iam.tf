resource "scaleway_iam_application" "runtime" {
  name        = "${var.name_prefix}-runtime"
  description = "Runtime application identity for nvbes ${var.environment}."
}

resource "scaleway_iam_policy" "runtime_project_access" {
  name           = "${var.name_prefix}-runtime-project-access"
  description    = "Minimal runtime access for nvbes ${var.environment} compute nodes."
  application_id = scaleway_iam_application.runtime.id

  rule {
    project_ids = [var.project_id]
    permission_set_names = [
      "ObjectStorageFullAccess",
      "SecretManagerReadOnly",
      "SecretManagerSecretAccess",
    ]
  }
}

resource "scaleway_iam_api_key" "runtime" {
  application_id     = scaleway_iam_application.runtime.id
  default_project_id = var.project_id
  description        = "Runtime API key for nvbes ${var.environment}."
}

resource "scaleway_iam_application" "audit_anchor_signer" {
  count = var.enable_external_audit_archive ? 1 : 0

  name        = "${var.name_prefix}-audit-anchor-signer"
  description = "Production identity restricted to KMS signing and verification."
}

resource "scaleway_iam_policy" "audit_anchor_signer" {
  count = var.enable_external_audit_archive ? 1 : 0

  name           = "${var.name_prefix}-audit-anchor-signer"
  description    = "Can sign and verify anchor digests; cannot administer or export KMS key material."
  application_id = scaleway_iam_application.audit_anchor_signer[0].id

  rule {
    project_ids          = [var.project_id]
    permission_set_names = ["KeyManagerKeySign", "KeyManagerKeyVerify"]
  }
}

resource "scaleway_iam_api_key" "audit_anchor_signer" {
  count = var.enable_external_audit_archive ? 1 : 0

  application_id     = scaleway_iam_application.audit_anchor_signer[0].id
  default_project_id = var.project_id
  description        = "Rotating KMS-only credential for the signed audit anchor job."
}
