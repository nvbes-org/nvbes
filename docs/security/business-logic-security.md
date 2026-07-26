# Business Logic Security

This repository tracks Business Logic Security controls against the OWASP Business Logic Security Cheat Sheet. The source of truth is `docs/security/business-logic-security-controls.json`.

Business logic defects are treated as abuse paths through valid features, not as input-sanitization bugs. A protected workflow must therefore prove these properties:

- security-relevant values are derived from server-side state;
- sensitive workflows have explicit server-side state, expiry, and step checks;
- replays, duplicate submissions, and concurrent double execution are rejected;
- automation-friendly business features have rate limits and bot controls;
- authorization uses actor, resource, target, and policy context;
- values are validated for business meaning, not only JSON shape;
- denials and risky decisions are observable;
- invariants are pinned by automated checks.

The Account idempotency middleware implements the active race-control mechanism for keyed mutating requests. Before the business handler runs, it claims `nvbes:idem:claim:{scope}:{key}` in Redis with a request hash and owner token. A concurrent request with the same signature receives `409 idempotency_request_in_progress`; a concurrent request with a different body receives `422 idempotency_key_reuse`. The claim is released only after the response is stored for replay, and only if the stored owner token still matches.

Run:

```bash
pnpm check:business-logic-security
```

The check validates registry schema, unique IDs, referenced evidence files, and exact evidence strings.
