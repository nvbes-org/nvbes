# Event Contracts

Critical events use the common envelope in `envelope.schema.json` and a
versioned event schema listed in `manifest.json`.

Rules:

- `event_type` is stable and namespaced by domain;
- `event_version` increments for incompatible payload changes;
- producers publish through an outbox;
- consumers must be idempotent by `idempotency_key`;
- events containing personal data require classification before production.
