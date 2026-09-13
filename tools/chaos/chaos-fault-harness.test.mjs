import assert from 'node:assert/strict';
import test from 'node:test';
import {
  runChaosScenarios,
  simulateUpstreamFailure,
  testResilientHttpCall,
} from './chaos-fault-harness.mjs';

test('Chaos harness simulates upstream timeout cleanly', async () => {
  const upstream = await simulateUpstreamFailure({ port: 9281, faultMode: 'timeout' });
  try {
    const outcome = await testResilientHttpCall({
      targetUrl: upstream.url,
      timeoutMs: 100,
      maxRetries: 1,
    });
    assert.equal(outcome.success, false);
    assert.equal(outcome.attempts, 2);
  } finally {
    await upstream.close();
  }
});

test('Chaos harness executes all resilience scenarios and writes report', async () => {
  const report = await runChaosScenarios();
  assert.equal(report.passed, 3);
  assert.equal(report.scenariosTested, 3);
});
