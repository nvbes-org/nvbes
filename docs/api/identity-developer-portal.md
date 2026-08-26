# Developer Portal API

> **Status: historical future surface, outside active V1.** The referenced
> Developer runtimes are archived; Identity foundation work does not restore
> this portal without a separate product and FinOps decision.

The developer portal and Developer Console APIs are served by `developer-service` under
`/developer`. The service validates user access tokens through Account introspection and delegates
Identity-owned OAuth primitives to `account-service`; Developer RBAC, metadata, webhooks, sandbox,
health checks, and tooling remain owned by `developer-service`.

The generated contract is `apps/developer-service/openapi.json`. In local development the HTTP
surface listens on port `4040` and the internal gRPC surface on port `4041`.

## Permissions

| Permission                        | Purpose                                          |
| --------------------------------- | ------------------------------------------------ |
| `developer.apps.read`             | Read OAuth apps exposed in the developer portal. |
| `developer.apps.create`           | Create OAuth apps.                               |
| `developer.apps.update_redirects` | Update app redirect URIs.                        |
| `developer.apps.revoke`           | Revoke OAuth apps.                               |
| `developer.webhooks.read`         | Read webhook endpoints and delivery status.      |
| `developer.webhooks.manage`       | Create and delete webhook endpoints.             |
| `developer.logs.read`             | Read tenant-scoped developer logs.               |
| `developer.tokens.inspect`        | Inspect tokens without logging raw token bodies. |
| `developer.oauth.playground`      | Exchange OAuth playground authorization codes.   |
| `developer.rbac.manage`           | Assign and revoke developer roles.               |
| `developer.docs.read`             | Access connected docs-only portal views.         |

## Routes

| Route                                        | Permission                        |
| -------------------------------------------- | --------------------------------- |
| `GET /developer/me`                          | Authenticated developer context.  |
| `GET /developer/apps`                        | `developer.apps.read`             |
| `POST /developer/apps`                       | `developer.apps.create`           |
| `GET /developer/apps/{clientId}`             | `developer.apps.read`             |
| `PATCH /developer/apps/{clientId}/redirects` | `developer.apps.update_redirects` |
| `DELETE /developer/apps/{clientId}`          | `developer.apps.revoke`           |
| `GET /developer/webhooks`                    | `developer.webhooks.read`         |
| `POST /developer/webhooks`                   | `developer.webhooks.manage`       |
| `DELETE /developer/webhooks/{endpointId}`    | `developer.webhooks.manage`       |
| `POST /developer/tokens/inspect`             | `developer.tokens.inspect`        |
| `POST /developer/oauth/playground/exchange`  | `developer.oauth.playground`      |
| `GET /developer/logs`                        | `developer.logs.read`             |

## Webhook Events

- `user.created`
- `login.failed`
- `session.revoked`
- `client.created`

Webhook deliveries are signed. The signing secret is displayed only when created or rotated.
