import assert from 'node:assert/strict';
import { test } from 'node:test';
import { Instrumenter } from '@stryker-mutator/instrumenter';

const logger = { debug() {}, info() {}, warn() {}, isDebugEnabled: () => false };
const instrumenter = new Instrumenter(logger);

test('Stryker mutates executable expressions inside assertions but never type literals', async () => {
  const source = [
    "type Status = 'allowed' | 'denied';",
    "export const tokens = { enabled: true, label: 'Action' } as const;",
    'export const accepts = (a: boolean, b: boolean) => (a && b) as boolean;',
    "export const code = 'allowed' as Status;",
  ].join('\n');
  const result = await instrumenter.instrument(
    [{ name: 'probe.ts', content: source, mutate: true }],
    {
      plugins: null,
      ignorers: [],
      excludedMutations: [],
    },
  );
  const atLine = (line) =>
    result.mutants.filter((mutant) => mutant.location.start.line === line - 1);
  assert.equal(atLine(1).length, 0, 'Type aliases must not produce mutants');
  assert(atLine(2).some((m) => m.mutatorName === 'BooleanLiteral'));
  assert(atLine(2).some((m) => m.mutatorName === 'StringLiteral'));
  assert(
    atLine(3).some((m) => m.mutatorName === 'LogicalOperator' && m.replacement.includes('||')),
  );
  assert(atLine(4).some((m) => m.mutatorName === 'StringLiteral'));
});
