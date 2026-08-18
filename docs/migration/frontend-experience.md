# Frontend Experience Evidence

## Status

- status: passed
- checks: 34
- passed: 34
- failed: 0

## Rules

- Every evidence row must be generated from the frontend source contract.
- `passed` requires the configured file to contain the expected pattern.
- Summary counters must match evidence rows.
- Targeted tests must name the frontend checks required by the cutover gate.
- Generation provenance must identify sources, write command and targeted tests.

## Evidence

| Check | Status | Path |
|---|---:|---|
| Root web check formats Identity and Enterprise Web | passed | `package.json` |
| Root web check lints Identity and Enterprise Web | passed | `package.json` |
| Root web check typechecks Identity and Enterprise Web | passed | `package.json` |
| Root lint:web includes Identity and Enterprise Web | passed | `package.json` |
| Identity Web declares routed authentication journeys | passed | `apps/identity-web/src/identity.router.tsx` |
| Identity shell exposes a main landmark | passed | `apps/identity-web/src/components/AuthPageShell.tsx` |
| Identity Web has browser-level critical journey coverage | passed | `apps/identity-web/e2e/critical.spec.ts` |
| Identity Web verifies PAR, consent and PKCE in the hosted OAuth journey | passed | `apps/identity-web/e2e/critical.spec.ts` |
| Identity Web seeds fixtures through Identity Service | passed | `apps/identity-web/e2e/critical.spec.ts` |
| Root critical E2E runner executes Identity Web | passed | `scripts/test-e2e-critical.sh` |
| Root exposes the isolated Identity browser harness through Nx | passed | `package.json` |
| Identity critical journeys start isolated infrastructure and runtimes | passed | `scripts/test-identity-e2e-local.sh` |
| Identity Web tests universal login API behavior | passed | `apps/identity-web/src/identity.universal-login.test.js` |
| Identity Web tests universal login hook state | passed | `apps/identity-web/src/pages/useUniversalLogin.test.js` |
| Cloud Web declares routed file journeys | passed | `apps/cloud-web/src/drive.router.tsx` |
| Cloud Web shell exposes a main landmark | passed | `apps/cloud-web/src/DriveAppLayout.tsx` |
| Cloud Web exposes share copy and revoke actions | passed | `apps/cloud-web/src/DriveSharedLinksView.tsx` |
| Cloud Web tests upload drop handling | passed | `apps/cloud-web/src/drive.uploads.drop.test.ts` |
| Cloud Web tests session storage | passed | `apps/cloud-web/src/drive.session.storage.test.ts` |
| Cloud Web tests workspace switching state | passed | `apps/cloud-web/src/drive.workspace.store.test.ts` |
| Console Web declares routed console and portal journeys | passed | `apps/console-web/src/developer.router.tsx` |
| Developer console shell exposes a main landmark | passed | `apps/console-web/src/layouts/DeveloperShell.tsx` |
| Developer portal shell exposes a main landmark | passed | `apps/console-web/src/layouts/DeveloperPortalLayout.tsx` |
| Developer public shell exposes a main landmark | passed | `apps/console-web/src/layouts/DeveloperPublicLayout.tsx` |
| Console Web tests permission gating | passed | `apps/console-web/src/__tests__/developer.permissions.test.ts` |
| Console Web tests session handling | passed | `apps/console-web/src/__tests__/developer.session.test.ts` |
| Console Web tests webhook replay gating | passed | `apps/console-web/src/__tests__/developer.webhooks.test.ts` |
| Enterprise Web declares routed admin journeys | passed | `apps/enterprise-web/src/enterprise.router.tsx` |
| Enterprise Web shell exposes a main landmark | passed | `apps/enterprise-web/src/components/EnterpriseLayout.tsx` |
| Enterprise Web shell exposes navigation | passed | `apps/enterprise-web/src/components/EnterpriseSidebar.tsx` |
| Enterprise Web tests permission gating | passed | `apps/enterprise-web/src/enterprise.permissions.test.ts` |
| Enterprise Web tests invitation behavior | passed | `apps/enterprise-web/src/enterprise.invites.test.ts` |
| Cloud documentation boundary is documented | passed | `docs/cloud/README.md` |
| Backoffice frontend boundary is documented | passed | `apps/backoffice-service/README.md` |

## Decision

Frontend repository evidence is covered for Identity, Account, Cloud, Developer, Enterprise and Backoffice boundaries. Production cutover still requires the G4 strict E2E and AA accessibility sign-off.

## Regeneration

```bash
pnpm check:migration-frontend-experience
node tools/migration/frontend-experience.mjs --write
```
