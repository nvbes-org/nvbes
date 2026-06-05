/**
 * Bot Guard — Signals orchestrator
 *
 * Usage:
 *   const collector = createSignalsCollector(emailInput, submitButton);
 *   // ... user fills the form ...
 *   const signals = await collector.collect(); // call just before submit
 *   collector.destroy();                       // clean up listeners
 *
 * The returned `BotSignals` object is sent as `bot_signals` in the
 * POST /challenge/identifier payload alongside `bot_guard`.
 */

import { type AutomationSignals, collectAutomationSignals } from './bot-guard.signals.automation';
import { attachBehavioralObserver, type BehavioralSignals } from './bot-guard.signals.behavioral';
import {
  collectEnvironmentSignals,
  type EnvironmentSignals,
} from './bot-guard.signals.environment';
import { collectExtensionSignals, type ExtensionSignals } from './bot-guard.signals.extensions';
import {
  collectFingerprintSignals,
  type FingerprintSignals,
} from './bot-guard.signals.fingerprint';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type BotSignals = BehavioralSignals &
  EnvironmentSignals &
  FingerprintSignals &
  AutomationSignals &
  ExtensionSignals;

export interface SignalsCollector {
  /**
   * Collects and merges all signals.
   * Call this just before form submission — it triggers async probes (rAF, fonts).
   *
   * @param afterUserInteraction - Set true once the user has clicked or typed
   *   (required for accurate speech synthesis probe).
   */
  collect: (afterUserInteraction?: boolean) => Promise<BotSignals>;

  /** Frees all event listeners. Call when the form unmounts. */
  destroy: () => void;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Creates a signals collector attached to the given form elements.
 *
 * @param emailInput   - The email/identifier `<input>` element.
 * @param submitButton - The form submit `<button>` element.
 */
export function createSignalsCollector(
  emailInput: HTMLInputElement,
  submitButton: HTMLElement,
): SignalsCollector {
  const behavioral = attachBehavioralObserver(emailInput, submitButton);

  return {
    collect: async (afterUserInteraction = true): Promise<BotSignals> => {
      const [behavioralSnap, environment, fingerprint, extensions] = await Promise.all([
        Promise.resolve(behavioral.collect()),
        collectEnvironmentSignals(afterUserInteraction),
        collectFingerprintSignals(),
        collectExtensionSignals(),
      ]);

      const automation = collectAutomationSignals();

      return {
        ...behavioralSnap,
        ...environment,
        ...fingerprint,
        ...extensions,
        ...automation,
      };
    },
    destroy: () => {
      behavioral.destroy();
    },
  };
}

// ---------------------------------------------------------------------------
// Re-exports for convenience
// ---------------------------------------------------------------------------

export { collectAutomationSignals } from './bot-guard.signals.automation';
export { attachBehavioralObserver } from './bot-guard.signals.behavioral';
export { collectEnvironmentSignals } from './bot-guard.signals.environment';
export { collectExtensionSignals } from './bot-guard.signals.extensions';
export { collectFingerprintSignals } from './bot-guard.signals.fingerprint';
export type {
  AutomationSignals,
  BehavioralSignals,
  EnvironmentSignals,
  ExtensionSignals,
  FingerprintSignals,
};
