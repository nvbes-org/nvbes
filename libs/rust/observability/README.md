# nvbes Observability

Primitives transverses d'observabilité pour les services Rust de la plateforme nvbes.

## Fonctionnalités

- Tracing structuré via `tracing` et exportation OpenTelemetry (OTLP / gRPC).
- Collecte et exposition de métriques Prometheus (`/metrics`).
- Intégration Sentry pour le signalement des erreurs non gérées et profiling Pyroscope.
