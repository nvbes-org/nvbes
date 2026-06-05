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
