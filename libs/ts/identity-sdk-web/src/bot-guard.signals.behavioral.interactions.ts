const CASCADE_BITS = {
  pointerdown: 0,
  mousedown: 1,
  pointerup: 2,
  mouseup: 3,
  click: 4,
} as const;

export function trackEventCascade(submitButton: HTMLElement) {
  let cascade = 0;
  let keyboardSubmit = false;
  let lastEventBit = 0;
  let orderValid = true;
  let hasEvents = false;
  const handlers: [string, EventListener][] = [];

  const record = (bit: number) => {
    cascade |= 1 << bit;
    hasEvents = true;
    if (!keyboardSubmit && lastEventBit > bit) orderValid = false;
    lastEventBit = bit;
  };

  for (const [eventName, bit] of Object.entries(CASCADE_BITS)) {
    const handler = () => record(bit);
    submitButton.addEventListener(eventName, handler);
    handlers.push([eventName, handler]);
  }

  const onKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Enter') keyboardSubmit = true;
  };
  document.addEventListener('keydown', onKeydown);

  return {
    getCascade: () => cascade,
    getOrderValid: () => (hasEvents ? orderValid : null),
    wasKeyboardSubmit: () => keyboardSubmit,
    destroy: () => {
      for (const [eventName, handler] of handlers) {
        submitButton.removeEventListener(eventName, handler);
      }
      document.removeEventListener('keydown', onKeydown);
    },
  };
}

export function trackFocusSequence(input: HTMLInputElement) {
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
    getCaretAtEnd: () =>
      input.value.length === 0 ? null : input.selectionStart === input.value.length,
    destroy: () => input.removeEventListener('focusin', onFocus),
  };
}

export function trackScroll() {
  let count = 0;
  const speeds: number[] = [];
  let lastPosition = window.scrollY;
  let lastTimestamp = performance.now();
  const onScroll = () => {
    const currentTimestamp = performance.now();
    const elapsed = currentTimestamp - lastTimestamp;
    if (elapsed > 0) {
      speeds.push(Math.abs(window.scrollY - lastPosition) / elapsed);
      if (speeds.length > 30) speeds.shift();
    }
    lastPosition = window.scrollY;
    lastTimestamp = currentTimestamp;
    count++;
  };
  window.addEventListener('scroll', onScroll, { passive: true });

  return {
    getCount: () => count,
    getSpeedVariance: () => variance(speeds),
    destroy: () => window.removeEventListener('scroll', onScroll),
  };
}

export function trackVisibility(element: HTMLElement) {
  let visible: boolean | null = null;
  if (typeof IntersectionObserver === 'undefined') {
    return { getVisible: () => null, destroy: () => undefined };
  }

  const observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting) visible = true;
    },
    { threshold: 0.1 },
  );
  observer.observe(element);

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

function variance(values: number[]): number {
  if (values.length < 2) return 0;
  const average = values.reduce((sum, value) => sum + value, 0) / values.length;
  return values.reduce((sum, value) => sum + (value - average) ** 2, 0) / values.length;
}
