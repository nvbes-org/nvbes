import test from 'node:test';
import assert from 'node:assert/strict';
import { findUpstreamViolationsInDiff } from './check-upstream-rules.mjs';

test('detects SQL UPDATE without WHERE', () => {
  const diff = `
diff --git a/migrations/001.sql b/migrations/001.sql
--- a/migrations/001.sql
+++ b/migrations/001.sql
@@ -10,0 +11,1 @@
+UPDATE users SET active = false;
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'plsql:DeleteOrUpdateWithoutWhereCheck');
});

test('allows SQL UPDATE with explicit WHERE', () => {
  const diff = `
diff --git a/migrations/001.sql b/migrations/001.sql
--- a/migrations/001.sql
+++ b/migrations/001.sql
@@ -10,0 +11,1 @@
+UPDATE users SET active = false WHERE id = $1;
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 0);
});

test('detects TypeScript sort without comparator', () => {
  const diff = `
diff --git a/src/app.ts b/src/app.ts
--- a/src/app.ts
+++ b/src/app.ts
@@ -5,0 +6,1 @@
+const sorted = items.sort();
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'typescript:S2871');
});

test('allows TypeScript sort with comparator', () => {
  const diff = `
diff --git a/src/app.ts b/src/app.ts
--- a/src/app.ts
+++ b/src/app.ts
@@ -5,0 +6,1 @@
+const sorted = items.sort((a, b) => a.localeCompare(b));
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 0);
});

test('detects Math.random in production TypeScript code', () => {
  const diff = `
diff --git a/src/token.ts b/src/token.ts
--- a/src/token.ts
+++ b/src/token.ts
@@ -20,0 +21,1 @@
+const nonce = Math.random().toString(36);
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'typescript:S2245');
});

test('ignores Math.random in test files', () => {
  const diff = `
diff --git a/src/token.test.ts b/src/token.test.ts
--- a/src/token.test.ts
+++ b/src/token.test.ts
@@ -20,0 +21,1 @@
+const nonce = Math.random().toString(36);
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 0);
});

test('detects global parseInt', () => {
  const diff = `
diff --git a/src/util.ts b/src/util.ts
--- a/src/util.ts
+++ b/src/util.ts
@@ -10,0 +11,1 @@
+const num = parseInt(val, 10);
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'typescript:S7773');
});

test('allows Number.parseInt', () => {
  const diff = `
diff --git a/src/util.ts b/src/util.ts
--- a/src/util.ts
+++ b/src/util.ts
@@ -10,0 +11,1 @@
+const num = Number.parseInt(val, 10);
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 0);
});

test('detects ReDoS regex pattern', () => {
  const diff = `
diff --git a/src/jwt.ts b/src/jwt.ts
--- a/src/jwt.ts
+++ b/src/jwt.ts
@@ -15,0 +16,1 @@
+const cleaned = raw.replace(/=+$/, '');
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'typescript:S8786');
});

test('detects global parseFloat', () => {
  const diff = `
diff --git a/src/math.ts b/src/math.ts
--- a/src/math.ts
+++ b/src/math.ts
@@ -10,0 +11,1 @@
+const val = parseFloat(str);
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'typescript:S7773');
});

test('detects Promise rejection with string or raw object', () => {
  const diff = `
diff --git a/src/async.ts b/src/async.ts
--- a/src/async.ts
+++ b/src/async.ts
@@ -10,0 +11,1 @@
+reject("failed to fetch");
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'typescript:S6671');
});

test('allows Promise rejection with Error instance', () => {
  const diff = `
diff --git a/src/async.ts b/src/async.ts
--- a/src/async.ts
+++ b/src/async.ts
@@ -10,0 +11,1 @@
+reject(new Error("failed to fetch"));
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 0);
});

test('detects Rust thread::sleep in production code', () => {
  const diff = `
diff --git a/apps/identity-service/src/service.rs b/apps/identity-service/src/service.rs
--- a/apps/identity-service/src/service.rs
+++ b/apps/identity-service/src/service.rs
@@ -10,0 +11,1 @@
+std::thread::sleep(std::time::Duration::from_secs(1));
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'rust:S7487');
});

test('detects GitHub Actions workflow ${{ github.event }} injection in run step', () => {
  const diff = `
diff --git a/.github/workflows/deploy.yml b/.github/workflows/deploy.yml
--- a/.github/workflows/deploy.yml
+++ b/.github/workflows/deploy.yml
@@ -10,0 +11,1 @@
+        run: echo "Event: \${{ github.event.issue.title }}"
`;
  const violations = findUpstreamViolationsInDiff(diff);
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, 'actions:CWE-078');
});
