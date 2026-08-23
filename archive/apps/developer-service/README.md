# Developer Service

`developer-service` owns the Developer Portal and Developer Console domain.

- HTTP API: `/developer/**`, default local port `4040`
- Internal gRPC API: default local port `4041`
- OpenAPI: `apps/developer-service/openapi.json`
- Authentication: bearer tokens validated through Account OAuth introspection
- OAuth primitives: delegated to `account-service`

Run locally with:

```bash
pnpm dev:developer-service
```

Required runtime configuration is documented in the repository `.env.example`.
