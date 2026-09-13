import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
  checkSourceImportBoundaries,
  sourceImportSpecifiers,
} from './check-nx-boundaries.imports.mjs';

test('extracts static, exported, dynamic, and CommonJS imports from the AST', () => {
  const imports = sourceImportSpecifiers(`
		import value from "@nvbes/value";
		export { other } from "../other";
		const lazy = import("./lazy");
		const commonJs = require("./common-js");
		const text = 'require("ignored-string")';
		// import "ignored-comment";
	`);

  assert.deepEqual(imports, ['@nvbes/value', '../other', './lazy', './common-js']);
});

test('rejects AST dependency violations and cross-project relative imports', () => {
  const workspace = mkdtempSync(join(tmpdir(), 'nvbes-import-boundaries-'));
  try {
    mkdirSync(join(workspace, 'libs/source/src'), { recursive: true });
    mkdirSync(join(workspace, 'libs/target/src'), { recursive: true });
    writeFileSync(
      join(workspace, 'libs/source/src/index.ts'),
      'import "@nvbes/target"; export * from "../../target/src/index";',
    );
    writeFileSync(join(workspace, 'libs/target/src/index.ts'), 'export const value = 1;');
    const tags = (...values) => new Set(values);
    const projects = new Map([
      [
        'source',
        {
          root: 'libs/source',
          tags: tags('scope:oss', 'type:lib', 'domain:shared', 'layer:ui'),
        },
      ],
      [
        'target',
        {
          root: 'libs/target',
          tags: tags('scope:oss', 'type:lib', 'domain:identity', 'layer:sdk'),
        },
      ],
    ]);
    const errors = [];
    const addErrors = (source, target, _projects, found) =>
      found.push(`${source} cannot depend on ${target}`);

    checkSourceImportBoundaries(
      projects,
      new Map([['@nvbes/target', 'target']]),
      addErrors,
      errors,
      workspace,
    );

    assert.deepEqual(errors, [
      'source cannot depend on target',
      'libs/source/src/index.ts: cross-project relative import ../../target/src/index bypasses the target public package boundary',
      'source cannot depend on target',
    ]);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});
