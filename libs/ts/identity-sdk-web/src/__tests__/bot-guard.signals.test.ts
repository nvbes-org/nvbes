/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from 'vite-plus/test';
import {
  collectAutomationSignals,
  collectEnvironmentSignals,
  collectExtensionSignals,
  collectFingerprintSignals,
  createSignalsCollector,
} from '../bot-guard.signals';

describe('bot-guard.signals', () => {
  it('collects automation signals correctly', () => {
    const automation = collectAutomationSignals();
    expect(typeof automation).toBe('object');
    expect(automation).not.toBeNull();
  });

  it('collects environment signals without throwing', async () => {
    const env = await collectEnvironmentSignals(false);
    expect(typeof env).toBe('object');
    expect(env).not.toBeNull();
  });

  it('collects fingerprint and extension signals', async () => {
    const [fp, ext] = await Promise.all([collectFingerprintSignals(), collectExtensionSignals()]);
    expect(typeof fp).toBe('object');
    expect(typeof ext).toBe('object');
  });

  it('orchestrates all signals via createSignalsCollector', async () => {
    const input = document.createElement('input');
    input.type = 'email';
    const button = document.createElement('button');
    document.body.appendChild(input);
    document.body.appendChild(button);

    const collector = createSignalsCollector(input, button);
    const signals = await collector.collect(false);

    expect(typeof signals).toBe('object');
    expect(signals).not.toBeNull();

    collector.destroy();
    input.remove();
    button.remove();
  });
});
