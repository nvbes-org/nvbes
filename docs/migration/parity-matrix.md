# Functional Parity Matrix

## Status

Initialized. Not signed for cutover.

- total_capabilities: 16
- covered_capabilities: 16
- pending_capabilities: 0
- critical_capabilities: 15

## Gate

Cutover is blocked until every critical row is `covered`, `removed with owner
approval`, or `accepted reject` with explicit evidence.

## Matrix

| Domain | Capability | Critical | Owner | Target evidence | Status |
|---|---|---:|---|---|---|
| Identity | register | yes | Identity owner | `docs/migration/identity-register.generated.json`, `cargo test -p nvbes-account-service register_input_from_request --locked`, and seeded OpenAPI smoke registration path | covered |
| Identity | login/logout/refresh | yes | Identity owner | `docs/migration/identity-login-session.generated.json`, `cargo test -p nvbes-account-service refreshed_cookies --locked`, `cargo test -p nvbes-account-service logout_state_revokes_current_session_and_refresh_token_only --locked`, and seeded OpenAPI smoke login path | covered |
| Identity | MFA/WebAuthn | yes | Identity owner | `docs/migration/identity-mfa-webauthn.generated.json`, `cargo test -p nvbes-account-service validate_single_factor --locked`, `cargo test -p nvbes-account-service shape_registration_options --locked`, and `cargo test -p nvbes-account-service login_methods_are_ordered_by_preference --locked` | covered |
| Workspace/Authz | membership roles | yes | Workspace/Authz owner | `docs/migration/workspace-membership-roles.generated.json`, `cargo test -p nvbes-core workspace_membership_roles_cover_expected_permission_boundaries --locked`, and `cargo test -p nvbes-account-service permission_denied_metadata_captures_role_action_and_target_role --locked` | covered |
| Workspace/Authz | last-owner protection | yes | Workspace/Authz owner | `docs/migration/workspace-last-owner.generated.json`, `cargo test -p nvbes-enterprise-service last_owner_guard --locked`, and `cargo test -p nvbes-core owner_cannot_invite_or_remove_another_owner --locked` | covered |
| Drive | upload/download | yes | Drive owner | `docs/migration/drive-upload-download.generated.json`, `cargo test -p nvbes-cloud-service upload --locked`, `cargo test -p nvbes-cloud-service download --locked`, `cargo test -p nvbes-cloud-service resolve_range --locked`, and object-storage reconciliation jobs | covered |
| Drive | share/revoke | yes | Drive owner | `docs/migration/drive-share-revoke.generated.json`, `cargo test -p nvbes-cloud-service share_link --locked`, and share-link reconciliation report | covered |
| Drive | quotas | yes | Drive owner | `docs/migration/drive-quotas.generated.json`, `cargo test -p nvbes-cloud-service quota --locked`, `cargo test -p nvbes-cloud-worker recalculate_quotas_updates_used_storage_bytes --locked`, and `pnpm check:migration-reconciliation-report` | covered |
| Billing/Usage | entitlements | yes | Billing/Usage owner | `docs/migration/billing-entitlements.generated.json`, `cargo test -p nvbes-billing entitlements_view --locked`, `cargo test -p nvbes-billing build_invoice_estimate_charges_only_billable_overages --locked`, and `pnpm check:migration-reconciliation-report` | covered |
| Billing/Usage | webhooks | yes | Billing/Usage owner | `docs/migration/billing-webhook-idempotency.generated.json` and `cargo test -p nvbes-billing classify_webhook_retry --locked` | covered |
| Billing/Usage | multi-PSP continuity | yes | Billing/Usage owner | `docs/migration/billing-multi-psp-continuity.generated.json`, `docs/migration/billing-multi-psp-e2e.generated.json`, `cargo test -p nvbes-billing provider_subscription --locked`, `cargo test -p nvbes-billing provider_code --locked`, and `cargo test -p nvbes-billing workspace_effects_apply_only_to_primary_provider_subscription --locked` | covered |
| Audit/Privacy | audit append-only | yes | Audit/Privacy owner | `docs/migration/audit-append-only.generated.json` and `cargo test -p nvbes-audit --locked` | covered |
| Audit/Privacy | export/delete requests | yes | Audit/Privacy owner | `docs/migration/privacy-export-delete.generated.json`, `cargo test -p nvbes-account-worker data_export_worker_payload --locked`, and `cargo test -p nvbes-account-service account_export --locked` | covered |
| Developer Platform | OAuth apps/tokens | yes | Developer Platform owner | `docs/migration/developer-oauth-tokens.generated.json`, `cargo test -p nvbes-developer-service token_inspection_result_is_recorded_for_actor_and_tenant --locked`, and SDK generation | covered |
| Developer Platform | signed webhooks | yes | Developer Platform owner | `docs/migration/developer-signed-webhooks.generated.json`, `cargo test -p nvbes-developer-service api_log_reads_and_replay_use_webhook_deliveries --locked`, and console-web replay gating test | covered |
| Cloud/Internal | provisioning console | no | Cloud/Internal owner | `docs/migration/cloud-provisioning.generated.json`, `go test ./libs/go/provisioning ./libs/go/control-plane`, and Cloud-only OSS exclusion checks | covered |

## Review Rules

- `pending` means not ready for cutover.
- `covered` requires an owner and automated evidence.
- `accepted reject` requires owner, reason and user impact.
- `removed` requires a documented product decision.
