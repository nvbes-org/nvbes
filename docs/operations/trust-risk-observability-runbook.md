# Trust/Risk observability runbook

The production Trust/Risk runtime exports error events to Sentry and OTLP traces
and metrics to Grafana Cloud. It remains scale-to-zero, so an absence of data
during idle periods is expected and must not page operators.

## Error rate

1. Confirm the alert window contains real Trust/Risk traffic.
2. Open the matching Grafana trace and copy its trace ID.
3. Search Sentry for the same release, environment and trace ID.
4. Distinguish rejected client input from unavailable database or projection
   operations. Client validation errors are expected and must not be reported to
   Sentry.
5. If the failure started with a release, roll back to the previous signed image
   digest and preserve the failing trace and Sentry event for analysis.

## Latency

1. Compare request span duration with database child spans.
2. Check Serverless SQL cold-start state before increasing capacity; cold starts
   are an accepted consequence of `cpu_min = 0`.
3. Review projection backlog and assessment duration metrics by recommendation.
4. Do not raise the minimum container or database scale. Optimize queries,
   indexes or batching while preserving the scale-to-zero policy.

## Credential rotation

- Rotate the Sentry DSN and Grafana tokens through the protected GitHub
  environment only.
- Keep Grafana access-policy scopes limited to OTLP metrics/traces ingestion.
- Keep the Grafana provisioning service account limited to dashboards and
  alerting resources.
- Redeploy and run both observability smoke checks before revoking the previous
  credentials.
