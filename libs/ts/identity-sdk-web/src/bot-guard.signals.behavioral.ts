import {
  trackEventCascade,
  trackFocusSequence,
  trackScroll,
  trackVisibility,
} from './bot-guard.signals.behavioral.interactions';
import { trackKeyboardBiometrics, trackMouseEntropy } from './bot-guard.signals.behavioral.input';
import type { BehavioralObserver, BehavioralSignals } from './bot-guard.signals.behavioral.types';

export type { BehavioralObserver, BehavioralSignals } from './bot-guard.signals.behavioral.types';

export function attachBehavioralObserver(
  emailInput: HTMLInputElement,
  submitButton: HTMLElement,
): BehavioralObserver {
  const mouse = trackMouseEntropy();
  const keyboard = trackKeyboardBiometrics(emailInput);
  const cascade = trackEventCascade(submitButton);
  const focus = trackFocusSequence(emailInput);
  const scroll = trackScroll();
  const visibility = trackVisibility(submitButton);

  return {
    collect: (): BehavioralSignals => ({
      mouse_event_count: mouse.getCount(),
      mouse_variance: mouse.getVariance(),
      mouse_path_length: mouse.getPathLength(),
      kb_dwell_variance: keyboard.getDwellVariance(),
      kb_dwell_mean: keyboard.getDwellMean(),
      kb_flight_variance: keyboard.getFlightVariance(),
      kb_flight_mean: keyboard.getFlightMean(),
      kb_sample_count: keyboard.getSampleCount(),
      event_cascade: cascade.getCascade(),
      event_order_valid: cascade.getOrderValid(),
      keyboard_submit: cascade.wasKeyboardSubmit(),
      email_had_focus: focus.getHadFocus(),
      email_focus_before_value: focus.getFocusBeforeValue(),
      caret_at_end: focus.getCaretAtEnd(),
      scroll_event_count: scroll.getCount(),
      scroll_speed_variance: scroll.getSpeedVariance(),
      submit_visible: visibility.getVisible(),
    }),
    destroy: () => {
      mouse.destroy();
      keyboard.destroy();
      cascade.destroy();
      focus.destroy();
      scroll.destroy();
      visibility.destroy();
    },
  };
}
