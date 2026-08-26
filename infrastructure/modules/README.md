# Modules

Modules IaC reutilisables partages entre environnements.

## `scaleway-v1`

Module historique pour une infrastructure produit. La V1 active n'est pas une
production Drive ; toute utilisation doit respecter le scale-to-zero et le
budget global défini dans [`finops/`](../finops/README.md).

- Private Network regional.
- Security groups API et worker.
- Instance API publique derriere Cloudflare.
- Instance worker non exposee hors SSH allowliste.
- Managed PostgreSQL attache au reseau prive.
- Bucket Object Storage prive avec versioning et lifecycle rules.
- IAM application + API key runtime.

Le module ne stocke pas les valeurs applicatives secretes dans Secret Manager. Les overlays exposent un inventaire de secrets a remplir via la CI ou une procedure d'exploitation separee.
