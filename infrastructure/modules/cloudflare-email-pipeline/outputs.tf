output "worker_script_name" {
  value       = cloudflare_worker_script.email_worker.name
  description = "Nom du script Cloudflare Worker déployé"
}

output "scaleway_tem_domain_status" {
  value       = scaleway_tem_domain.main.status
  description = "Statut de vérification du domaine Scaleway TEM"
}

output "worker_endpoint" {
  value       = "https://email-${var.environment}.${var.domain_name}"
  description = "URL publique d'accès au Cloudflare Email Worker"
}
