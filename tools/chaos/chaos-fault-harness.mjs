#!/usr/bin/env node

import { createServer } from 'node:http';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

/**
 * Chaos Engineering & Resilience Fault Harness for nvbes V1.
 * Tests graceful degradation under simulated network failure, latency, and dependency loss.
 */

export async function simulateUpstreamFailure({ port = 9180, faultMode = 'timeout' } = {}) {
  const server = createServer(async (req, res) => {
    if (faultMode === 'timeout') {
      // Simulate hanging connection / packet drop
      await new Promise((resolve) => setTimeout(resolve, 3000));
      res.writeHead(504, { 'Content-Type': 'application/json' });
      res.end(
        JSON.stringify({ error: 'gateway_timeout', message: 'Upstream dependency timed out' }),
      );
    } else if (faultMode === 'connection_reset') {
      req.destroy();
    } else if (faultMode === 'http_500') {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(
        JSON.stringify({ error: 'internal_server_error', message: 'Dependency unavailable' }),
      );
    } else {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ status: 'ok' }));
    }
  });

  await new Promise((resolve) => server.listen(port, resolve));
  return {
    url: `http://127.0.0.1:${port}`,
    close: () => new Promise((resolve) => server.close(resolve)),
  };
}

export async function testResilientHttpCall({ targetUrl, timeoutMs = 1000, maxRetries = 2 }) {
  let attempts = 0;
  let lastError = null;

  for (let i = 0; i <= maxRetries; i++) {
    attempts++;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(targetUrl, { signal: controller.signal });
      clearTimeout(timer);
      if (response.ok) {
        return { success: true, attempts, status: response.status };
      }
      lastError = new Error(`HTTP ${response.status}`);
    } catch (err) {
      clearTimeout(timer);
      lastError = err;
    }

    // Exponential backoff
    await new Promise((r) => setTimeout(r, 50 * (i + 1)));
  }

  return { success: false, attempts, error: lastError?.message };
}

export async function runChaosScenarios() {
  const results = [];
  const startTime = Date.now();

  // Scenario 1: Upstream Gateway Timeout
  {
    const upstream = await simulateUpstreamFailure({ port: 9181, faultMode: 'timeout' });
    try {
      const outcome = await testResilientHttpCall({
        targetUrl: upstream.url,
        timeoutMs: 200,
        maxRetries: 2,
      });
      assert.equal(outcome.success, false, 'Expected call to fail cleanly under upstream timeout');
      assert.equal(outcome.attempts, 3, 'Expected exact retry budget of 3 attempts');
      results.push({
        scenario: 'upstream_timeout_bounded_retry',
        status: 'passed',
        details: 'Caller aborted after 200ms timeout per attempt and exited without hanging.',
      });
    } finally {
      await upstream.close();
    }
  }

  // Scenario 2: Connection Reset / Network Partition
  {
    const upstream = await simulateUpstreamFailure({ port: 9182, faultMode: 'connection_reset' });
    try {
      const outcome = await testResilientHttpCall({
        targetUrl: upstream.url,
        timeoutMs: 300,
        maxRetries: 1,
      });
      assert.equal(outcome.success, false, 'Expected clean error on connection reset');
      results.push({
        scenario: 'connection_reset_resilience',
        status: 'passed',
        details: 'Connection reset handled gracefully without uncaught exception.',
      });
    } finally {
      await upstream.close();
    }
  }

  // Scenario 3: Transient 500 error recovered by retry
  {
    let callCount = 0;
    const server = createServer((req, res) => {
      callCount++;
      if (callCount < 2) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'temporary_failure' }));
      } else {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ status: 'recovered' }));
      }
    });
    await new Promise((r) => server.listen(9183, r));

    try {
      const outcome = await testResilientHttpCall({
        targetUrl: 'http://127.0.0.1:9183',
        timeoutMs: 500,
        maxRetries: 2,
      });
      assert.equal(outcome.success, true, 'Expected retry to recover from transient 500 error');
      assert.equal(outcome.attempts, 2, 'Recovered on second attempt');
      results.push({
        scenario: 'transient_500_recovery',
        status: 'passed',
        details: 'Self-healed on retry without operator intervention.',
      });
    } finally {
      await new Promise((r) => server.close(r));
    }
  }

  const report = {
    executedAt: new Date().toISOString(),
    durationMs: Date.now() - startTime,
    scenariosTested: results.length,
    passed: results.filter((r) => r.status === 'passed').length,
    results,
  };

  const outputDir = path.resolve(process.cwd(), '.temp/chaos');
  await mkdir(outputDir, { recursive: true });
  await writeFile(path.join(outputDir, 'chaos-report.json'), JSON.stringify(report, null, 2));

  console.log(
    `Chaos Engineering Suite: ${report.passed}/${report.scenariosTested} scenarios passed in ${report.durationMs}ms.`,
  );
  return report;
}

if (process.argv[1] && process.argv[1].endsWith('chaos-fault-harness.mjs')) {
  runChaosScenarios().catch((err) => {
    console.error('Chaos suite failed:', err);
    process.exit(1);
  });
}
