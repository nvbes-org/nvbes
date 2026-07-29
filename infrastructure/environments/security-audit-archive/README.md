# Security audit archive

Ce stack est appliqué indépendamment de la production, avec un backend d’état,
un compte Scaleway et une identité CI détenus par Security. Il crée un bucket
Object Lock en mode `COMPLIANCE` pour sept ans et une identité limitée à
`PutObject` sous `anchors/`.

La production reçoit seulement:

- `audit_archive_bucket_name`;
- `writer_access_key`;
- `writer_secret_key`, transférée directement dans Secret Manager et jamais
  placée dans un fichier Terraform production.

Les credentials du provider et l’état de ce stack ne sont jamais accessibles au
compte production. Les changements de rétention, de policy ou d’identité
requièrent une revue Security indépendante.
