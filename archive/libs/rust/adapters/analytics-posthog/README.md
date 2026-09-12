# nvbes Analytics PostHog Adapter

Adaptateur Rust implémentant le trait `ProductAnalyticsClient` de `nvbes-product-analytics` via l'API PostHog.

## Fonctionnalités

- Envoi d'événements d'analytics produit non-bloquants vers l'instance PostHog configurée.
- Respect strict des politiques de consentement et d'anonymisation.
