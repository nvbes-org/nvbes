# nvbes DPoP

Implémentation Rust de la spécification OAuth 2.0 Demonstrating Proof-of-Possession (DPoP - RFC 9449).

## Fonctionnalités

- Construction et validation de preuves ES256 (signature, URL, méthode, date,
  empreinte du token). Une clé privée dans le header est refusée.
- Primitives de nonce séparées ; leur présence ne signifie pas qu'un endpoint
  HTTP a activé les challenges de nonce.
- Feature `resource-server` : origine canonique, liaison au token et registre
  PostgreSQL anti-rejeu borné, utilisé par Account et Billing. Voir le
  [contrat actif](../../../docs/architecture/identity-resource-dpop.md).
