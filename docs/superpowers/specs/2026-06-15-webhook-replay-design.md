# Webhook Replay Design

Allow developers to inspect webhook delivery attempts and replay failed or pending deliveries directly from the Developer Console.

## User Review Required

No breaking changes or high-risk actions are introduced. The backend endpoint `/developer/console/webhooks/deliveries/{deliveryId}/replay` is already defined and registered in the Axum router, so this design mainly details the frontend UI implementation and the integration testing.

## Proposed Changes

### Backend

#### [MODIFY] [identity.domains.developer.tests.rs](file:///Users/shayn/Development/nvbes/apps/account-service/src/identity.domains.developer.tests.rs)
- Add a new integration test `test_webhook_replay_workflow` that:
  - Sets up a tenant, a webhook endpoint, and a failed webhook delivery.
  - Calls the `replay_delivery` route.
  - Asserts that a new pending delivery record is created in the database, referencing the original delivery's `id` in its `replayed_from_delivery_id` column.

---

### Frontend

#### [MODIFY] [developer.api.ts](file:///Users/shayn/Development/nvbes/apps/console-web/src/developer.api.ts)
- Add `listDeveloperConsoleWebhookDeliveries` function.
- Add `replayDeveloperConsoleWebhookDelivery` function.

#### [MODIFY] [WebhooksPage.tsx](file:///Users/shayn/Development/nvbes/apps/console-web/src/pages/WebhooksPage.tsx)
- Upgrade the endpoint listing to support an expandable/accordion view.
- Introduce a sub-component `WebhookDeliveriesList` which:
  - Fetches delivery history for the given endpoint ID.
  - Displays a table/list of deliveries showing: status badge, event type, attempt count, response HTTP code, error message, and timestamp.
  - Checks if there are any pending deliveries in the fetched list. If so, enables polling by setting a `refetchInterval` of 2000ms.
  - Offers a "Replay" button next to each failed or pending delivery.
  - Performs the replay mutation and invalidates the queries on success to refresh the log.

---

## Verification Plan

### Automated Tests
- Run backend tests:
  ```bash
  rtk cargo test -p account-service domains::developer
  ```
- Run frontend tests:
  ```bash
  pnpm --filter console-web test
  ```

### Manual Verification
- Expand an endpoint card on the Webhooks page, view delivery logs, and verify clicking Replay triggers the state change successfully.
