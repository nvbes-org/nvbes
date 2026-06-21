# Frontend Experience Evidence

## Status

- status: passed
- checks: 29
- passed: 29
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
| Root web check formats Enterprise Web | passed | `package.json` |
| Root web check lints Enterprise Web | passed | `package.json` |
| Root web check typechecks Enterprise Web | passed | `package.json` |
| Root lint:web includes Enterprise Web | passed | `package.json` |
| Identity Web declares routed login/account journeys | passed | `apps/identity-web/src/identity.router.tsx` |
| Identity account shell exposes a main landmark | passed | `apps/identity-web/src/components/AccountLayout.tsx` |
| Identity Web has browser-level critical journey coverage | passed | `apps/identity-web/e2e/critical.spec.ts` |
| Identity Web tests universal login API behavior | passed | `apps/identity-web/src/identity.universal-login.test.js` |
| Identity Web tests universal login hook state | passed | `apps/identity-web/src/pages/useUniversalLogin.test.js` |
| Drive Web declares routed file journeys | passed | `apps/drive-web/src/drive.router.tsx` |
| Drive Web shell exposes a main landmark | passed | `apps/drive-web/src/DriveAppLayout.tsx` |
| Drive Web exposes share copy and revoke actions | passed | `apps/drive-web/src/DriveSharedLinksView.tsx` |
| Drive Web tests upload drop handling | passed | `apps/drive-web/src/drive.uploads.drop.test.ts` |
| Drive Web tests session storage | passed | `apps/drive-web/src/drive.session.storage.test.ts` |
| Drive Web tests workspace switching state | passed | `apps/drive-web/src/drive.workspace.store.test.ts` |
| Developer Web declares routed console and portal journeys | passed | `apps/developer-web/src/developer.router.tsx` |
| Developer console shell exposes a main landmark | passed | `apps/developer-web/src/layouts/DeveloperShell.tsx` |
| Developer portal shell exposes a main landmark | passed | `apps/developer-web/src/layouts/DeveloperPortalLayout.tsx` |
| Developer public shell exposes a main landmark | passed | `apps/developer-web/src/layouts/DeveloperPublicLayout.tsx` |
| Developer Web tests permission gating | passed | `apps/developer-web/src/__tests__/developer.permissions.test.ts` |
| Developer Web tests session handling | passed | `apps/developer-web/src/__tests__/developer.session.test.ts` |
| Developer Web tests webhook replay gating | passed | `apps/developer-web/src/__tests__/developer.webhooks.test.ts` |
| Enterprise Web declares routed admin journeys | passed | `apps/enterprise-web/src/enterprise.router.tsx` |
| Enterprise Web shell exposes a main landmark | passed | `apps/enterprise-web/src/components/EnterpriseLayout.tsx` |
| Enterprise Web shell exposes navigation | passed | `apps/enterprise-web/src/components/EnterpriseSidebar.tsx` |
| Enterprise Web tests permission gating | passed | `apps/enterprise-web/src/enterprise.permissions.test.ts` |
| Enterprise Web tests invitation behavior | passed | `apps/enterprise-web/src/enterprise.invites.test.ts` |
| Cloud Console frontend boundary is documented | passed | `apps/cloud-console/README.md` |
| Internal Admin frontend boundary is documented | passed | `apps/internal-admin/README.md` |

## Decision

Frontend repository evidence is covered for Identity, Drive, Developer, Enterprise, Cloud Console and Internal Admin boundaries. Production cutover still requires the G4 strict E2E and AA accessibility sign-off.

## Regeneration

```bash
pnpm check:migration-frontend-experience
tools/migration/frontend-experience.mjs --write
```
