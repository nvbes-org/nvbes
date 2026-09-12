# OpenAPI Contracts

`manifest.json` lists the REST API documents that must remain valid before a
release or migration gate can pass.

Rules:

- public APIs use `/v1`;
- every referenced document must parse as OpenAPI 3.x;
- every referenced document must have title, version and at least one path;
- every route source listed in the manifest must be covered by its OpenAPI
  document;
- SDK generation uses these documents as source inputs.
