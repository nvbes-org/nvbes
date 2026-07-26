/**
 * Bot Guard — Behavioral signals collector
 *
 * Attaches passive event listeners to the email input and submit button.
 * No user-visible side effects. All data is collected in memory and read
 * once at submission time via `collect()`.
 */

export interface BehavioralSignals {
  mouse_event_count: number;
  mouse_variance: number;
  mouse_path_length: number;
  kb_dwell_variance: number | null;
  kb_dwell_mean: number | null;
  kb_flight_variance: number | null;
  kb_flight_mean: number | null;
  kb_sample_count: number;
  /** 5-bit bitfield: bit0=pointerdown, 1=mousedown, 2=pointerup, 3=mouseup, 4=click */
  event_cascade: number;
  /** True if events arrived in correct order (down before up before click) */
  event_order_valid: boolean | null;
  keyboard_submit: boolean;
  email_had_focus: boolean;
  email_focus_before_value: boolean;
  caret_at_end: boolean | null;
  scroll_event_count: number;
  scroll_speed_variance: number;
  /** True if the submit button was visible (IntersectionObserver) */
  submit_visible: boolean | null;
}

export interface BehavioralObserver {
  collect: () => BehavioralSignals;
  destroy: () => void;
}

// ---------------------------------------------------------------------------
// Mouse entropy
// ---------------------------------------------------------------------------

function trackMouseEntropy(): {
  getCount: () => number;
  getVariance: () => number;
  getPathLength: () => number;
  destroy: () => void;
} {
  const isTouch = navigator.maxTouchPoints > 0;
  if (isTouch) {
    return {
      getCount: () => 0,
      getVariance: () => 999,
      getPathLength: () => 0,
      destroy: () => {},
    };
  }

  let count = 0;
  let pathLength = 0;
  const accelerations: number[] = [];
  let lastX = 0;
  let lastY = 0;
  let lastVx = 0;
  let lastVy = 0;
  let lastTs = 0;

  const onMove = (e: MouseEvent) => {
    const ts = e.timeStamp;
    const dt = ts - lastTs;
    if (dt > 0 && lastTs > 0) {
      const vx = (e.clientX - lastX) / dt;
      const vy = (e.clientY - lastY) / dt;
      const ax = Math.abs(vx - lastVx);
      const ay = Math.abs(vy - lastVy);
      accelerations.push(Math.sqrt(ax * ax + ay * ay));
      if (accelerations.length > 50) accelerations.shift();
      lastVx = vx;
      lastVy = vy;
      pathLength += Math.sqrt((e.clientX - lastX) ** 2 + (e.clientY - lastY) ** 2);
    }
    lastX = e.clientX;
    lastY = e.clientY;
    lastTs = ts;
    count++;
  };

  document.addEventListener('mousemove', onMove, { passive: true });

  const getVariance = () => {
    if (accelerations.length < 3) return 0;
    const mean = accelerations.reduce((s, v) => s + v, 0) / accelerations.length;
    const sq = accelerations.reduce((s, v) => s + (v - mean) ** 2, 0);
    return sq / accelerations.length;
  };

  return {
    getCount: () => count,
    getVariance,
    getPathLength: () => pathLength,
    destroy: () => document.removeEventListener('mousemove', onMove),
  };
}

// ---------------------------------------------------------------------------
// Keyboard biometrics
// ---------------------------------------------------------------------------

function trackKeyboardBiometrics(input: HTMLInputElement): {
  getDwellVariance: () => number | null;
  getDwellMean: () => number | null;
  getFlightVariance: () => number | null;
  getFlightMean: () => number | null;
  getSampleCount: () => number;
  destroy: () => void;
} {
  const keyDownTimes = new Map<string, number>();
  const dwellTimes: number[] = [];
  const flightTimes: number[] = [];
  let lastKeyUpTs = 0;

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key.length === 1 || e.key === 'Backspace') {
      keyDownTimes.set(e.key + e.timeStamp, e.timeStamp);
      if (lastKeyUpTs > 0) {
        flightTimes.push(e.timeStamp - lastKeyUpTs);
      }
    }
  };

  const onKeyUp = (e: KeyboardEvent) => {
    const key = e.key;
    for (const [k, ts] of keyDownTimes) {
      if (k.startsWith(key)) {
        dwellTimes.push(e.timeStamp - ts);
        keyDownTimes.delete(k);
        break;
      }
    }
    lastKeyUpTs = e.timeStamp;
  };

  input.addEventListener('keydown', onKeyDown);
  input.addEventListener('keyup', onKeyUp);

  const variance = (arr: number[]) => {
    if (arr.length < 3) return null;
    const mean = arr.reduce((s, v) => s + v, 0) / arr.length;
    const sq = arr.reduce((s, v) => s + (v - mean) ** 2, 0);
    return sq / arr.length;
  };
  const mean = (arr: number[]) => {
    if (arr.length === 0) return null;
    return arr.reduce((s, v) => s + v, 0) / arr.length;
  };

  return {
    getDwellVariance: () => variance(dwellTimes),
    getDwellMean: () => mean(dwellTimes),
    getFlightVariance: () => variance(flightTimes),
    getFlightMean: () => mean(flightTimes),
    getSampleCount: () => Math.min(dwellTimes.length, flightTimes.length),
    destroy: () => {
      input.removeEventListener('keydown', onKeyDown);
      input.removeEventListener('keyup', onKeyUp);
    },
  };
}

// ---------------------------------------------------------------------------
// Event cascade on the submit button
// ---------------------------------------------------------------------------

const CASCADE_BITS = {
  pointerdown: 0,
  mousedown: 1,
  pointerup: 2,
  mouseup: 3,
  click: 4,
} as const;

function trackEventCascade(submitBtn: HTMLElement): {
  getCascade: () => number;
  getOrderValid: () => boolean | null;
  wasKeyboardSubmit: () => boolean;
  destroy: () => void;
} {
  let cascade = 0;
  let keyboardSubmit = false;
  let lastEventBits = 0;
  let orderValid = true;
  let hasEvents = false;

  const handlers: [string, EventListener][] = [];

  const record = (bit: number) => {
    cascade |= 1 << bit;
    hasEvents = true;
    if (!keyboardSubmit && lastEventBits > bit) {
      orderValid = false;
    }
    lastEventBits = bit;
  };

  for (const [event, bit] of Object.entries(CASCADE_BITS)) {
    const handler = () => record(bit);
    submitBtn.addEventListener(event, handler);
    handlers.push([event, handler]);
  }

  const onKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') keyboardSubmit = true;
  };
  document.addEventListener('keydown', onKeydown);

  return {
    getCascade: () => cascade,
    getOrderValid: () => (hasEvents ? orderValid : null),
    wasKeyboardSubmit: () => keyboardSubmit,
    destroy: () => {
      handlers.forEach(([ev, h]) => {
        submitBtn.removeEventListener(ev, h);
      });
      document.removeEventListener('keydown', onKeydown);
    },
  };
}

// ---------------------------------------------------------------------------
// Focus / blur sequence on the email field
// ---------------------------------------------------------------------------

function trackFocusSequence(input: HTMLInputElement): {
  getHadFocus: () => boolean;
  getFocusBeforeValue: () => boolean;
  getCaretAtEnd: () => boolean | null;
  destroy: () => void;
} {
  let hadFocus = false;
  let focusBeforeValue = false;

  const onFocus = () => {
    hadFocus = true;
    if (!input.value) focusBeforeValue = true;
  };

  input.addEventListener('focusin', onFocus);

  return {
    getHadFocus: () => hadFocus,
    getFocusBeforeValue: () => focusBeforeValue,
    getCaretAtEnd: () => {
      if (!input.value || input.value.length === 0) return null;
      return input.selectionStart === input.value.length;
    },
    destroy: () => input.removeEventListener('focusin', onFocus),
  };
}

// ---------------------------------------------------------------------------
// Scroll event counter
// ---------------------------------------------------------------------------

function trackScroll(): {
  getCount: () => number;
  getSpeedVariance: () => number;
  destroy: () => void;
} {
  let count = 0;
  const speeds: number[] = [];
  let lastY = window.scrollY;
  let lastTs = performance.now();
  const onScroll = () => {
    const now = performance.now();
    const dt = now - lastTs;
    if (dt > 0) {
      const dy = Math.abs(window.scrollY - lastY);
      speeds.push(dy / dt);
      if (speeds.length > 30) speeds.shift();
    }
    lastY = window.scrollY;
    lastTs = now;
    count++;
  };
  window.addEventListener('scroll', onScroll, { passive: true });
  return {
    getCount: () => count,
    getSpeedVariance: () => {
      if (speeds.length < 2) return 0;
      const mean = speeds.reduce((s, v) => s + v, 0) / speeds.length;
      const sq = speeds.reduce((s, v) => s + (v - mean) ** 2, 0);
      return sq / speeds.length;
    },
    destroy: () => window.removeEventListener('scroll', onScroll),
  };
}

// ---------------------------------------------------------------------------
// IntersectionObserver — validate submit button was visible
// ---------------------------------------------------------------------------

function trackVisibility(element: HTMLElement): {
  getVisible: () => boolean | null;
  destroy: () => void;
} {
  let visible: boolean | null = null;

  if (typeof IntersectionObserver === 'undefined') {
    return { getVisible: () => null, destroy: () => {} };
  }

  const observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting) visible = true;
    },
    { threshold: 0.1 },
  );
  observer.observe(element);

  // After 2s, if still null, element never intersected
  const timeoutId = setTimeout(() => {
    if (visible === null) visible = false;
    observer.disconnect();
  }, 2000);

  return {
    getVisible: () => visible,
    destroy: () => {
      clearTimeout(timeoutId);
      observer.disconnect();
    },
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Attaches all behavioral observers to the given form elements.
 * Call `destroy()` when the form unmounts to free listeners.
 */
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
