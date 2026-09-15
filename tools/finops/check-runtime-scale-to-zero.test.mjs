import assert from 'node:assert/strict';
import test from 'node:test';
import { verifyRuntimeScaleToZero } from './check-runtime-scale-to-zero.mjs';

test('verifyRuntimeScaleToZero validates email production delivery config', async () => {
  const result = await verifyRuntimeScaleToZero(
    'infrastructure/environments/email-production/email-delivery.tf',
  );
  assert.equal(result.valid, true);
  assert.equal(result.violations.length, 0);
});
