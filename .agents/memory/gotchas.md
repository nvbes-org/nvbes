# Gotcha Memory

- The worktree may be dirty. Do not revert unrelated user changes.
- `package.json` contains many validation scripts; prefer adding narrow scripts
  without disrupting existing chains.
- Some agent skill directories are large. Use the registry and routing docs
  before loading broad skill trees.
- **SonarQube / CodeQL Upstream Rules**:
  - SQL: Never write `UPDATE` or `DELETE` without a `WHERE` clause in migrations or SQL files (`plsql:DeleteOrUpdateWithoutWhereCheck`).
  - JS/TS: Always pass an explicit comparator to `.sort()` (`typescript:S2871`).
  - JS/TS: Never reject Promises with non-Error objects (`typescript:S6671`).
  - JS/TS: Avoid catastrophic regexes (e.g. `/=+$/` -> `/={1,2}$/`) (`typescript:S8786`).
  - JS/TS: Never use `Math.random()` in security/token/crypto/fingerprint contexts; use `crypto.getRandomValues()` (`typescript:S2245`).
  - JS/TS: Always use `Number.parseInt()` instead of global `parseInt()` (`typescript:S7773`).
  - Rust: Never make blocking calls (`child.wait()`) in async Tokio contexts (`rust:S7487`).
  - Coverage: SonarCloud requires >= 80% coverage on new code. In LCOV reports, source paths must be relative to repository root (`SF:apps/...` or `SF:libs/...`).
  - Commits: Conventional Commits with body line length < 80 chars, and trailer `AI-Assisted: <agent>`.

