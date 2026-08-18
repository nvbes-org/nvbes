terraform {
  backend "s3" {
    key     = "production/bootstrap/terraform.tfstate"
    region  = "fr-par"
    encrypt = true

    endpoints = {
      s3 = "https://s3.fr-par.scw.cloud"
    }

    skip_credentials_validation = true
    skip_region_validation      = true
    skip_requesting_account_id  = true
    skip_metadata_api_check     = true
    use_lockfile                = true
  }
}
