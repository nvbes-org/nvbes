// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { trackKeyboardBiometrics, trackMouseEntropy } from './bot-guard.signals.behavioral.input';
import {
  trackEventCascade,
  trackFocusSequence,
  trackScroll,
  trackVisibility,
} from './bot-guard.signals.behavioral.interactions';
import { attachBehavioralObserver } from './bot-guard.signals.behavioral';

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal('navigator', { maxTouchPoints: 0 });
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function key(input: HTMLInputElement, type: string, value: string, timestamp: number) {
  const event = new KeyboardEvent(type, { key: value });
  Object.defineProperty(event, 'timeStamp', { value: timestamp });
  input.dispatchEvent(event);
}
function mouse(x: number, timestamp: number) {
  const event = new MouseEvent('mousemove', { clientX: x, clientY: 0 });
  Object.defineProperty(event, 'timeStamp', { value: timestamp });
  document.dispatchEvent(event);
}

it('measures keyboard dwell and flight intervals and detaches both listeners', () => {
  const input = document.createElement('input');
  const tracker = trackKeyboardBiometrics(input);
  expect(tracker.getDwellMean()).toBeNull();
  expect(tracker.getFlightMean()).toBeNull();
  expect(tracker.getDwellVariance()).toBeNull();
  expect(tracker.getFlightVariance()).toBeNull();
  key(input, 'keydown', 'Shift', 1);
  for (const [value, down, up] of [
    ['a', 10, 20],
    ['b', 40, 60],
    ['Backspace', 90, 120],
    ['c', 160, 200],
  ] as const) {
    key(input, 'keydown', value, down);
    key(input, 'keyup', value, up);
  }
  expect(tracker.getDwellMean()).toBe(25);
  expect(tracker.getDwellVariance()).toBe(125);
  expect(tracker.getFlightMean()).toBe(30);
  expect(tracker.getFlightVariance()).toBeCloseTo(200 / 3);
  expect(tracker.getSampleCount()).toBe(3);
  tracker.destroy();
  key(input, 'keydown', 'd', 220);
  key(input, 'keyup', 'd', 400);
  expect(tracker.getSampleCount()).toBe(3);
});
it('matches key-up with its own key when multiple keys overlap', () => {
  const input = document.createElement('input');
  const tracker = trackKeyboardBiometrics(input);
  key(input, 'keydown', 'a', 10);
  key(input, 'keydown', 'b', 20);
  key(input, 'keyup', 'b', 25);
  expect(tracker.getDwellMean()).toBe(5);
  key(input, 'keyup', 'a', 40);
  expect(tracker.getDwellMean()).toBe(17.5);
  tracker.destroy();
});
it('measures mouse path and acceleration without zero-time divisions', () => {
  const tracker = trackMouseEntropy();
  mouse(0, 10);
  mouse(10, 20);
  mouse(30, 30);
  mouse(60, 40);
  expect(tracker.getCount()).toBe(4);
  expect(tracker.getPathLength()).toBe(60);
  expect(tracker.getVariance()).toBe(0);
  mouse(60, 40);
  expect(tracker.getPathLength()).toBe(60);
  tracker.destroy();
  mouse(100, 50);
  expect(tracker.getCount()).toBe(5);
});
it('bounds mouse acceleration history to the latest 50 samples', () => {
  const tracker = trackMouseEntropy();
  mouse(0, 10);
  mouse(1000, 20);
  for (let i = 1; i <= 55; i++) mouse(1000 + i, 20 + i * 10);
  expect(tracker.getVariance()).toBe(0);
  expect(tracker.getPathLength()).toBe(1055);
  tracker.destroy();
});
it('does not attach mouse tracking on touch devices', () => {
  vi.stubGlobal('navigator', { maxTouchPoints: 1 });
  const tracker = trackMouseEntropy();
  mouse(100, 20);
  expect([tracker.getCount(), tracker.getVariance(), tracker.getPathLength()]).toEqual([0, 999, 0]);
  tracker.destroy();
});
it('tracks valid pointer cascades, invalid ordering and keyboard submission separately', () => {
  const button = document.createElement('button');
  const tracker = trackEventCascade(button);
  expect(tracker.getOrderValid()).toBeNull();
  for (const name of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click'])
    button.dispatchEvent(new Event(name));
  expect(tracker.getCascade()).toBe(31);
  expect(tracker.getOrderValid()).toBe(true);
  button.dispatchEvent(new Event('pointerdown'));
  expect(tracker.getOrderValid()).toBe(false);
  expect(tracker.wasKeyboardSubmit()).toBe(false);
  tracker.destroy();
  const keyboard = trackEventCascade(button);
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
  expect(keyboard.wasKeyboardSubmit()).toBe(false);
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
  button.dispatchEvent(new Event('click'));
  button.dispatchEvent(new Event('pointerdown'));
  expect(keyboard.wasKeyboardSubmit()).toBe(true);
  expect(keyboard.getOrderValid()).toBe(true);
  keyboard.destroy();
  button.dispatchEvent(new Event('mouseup'));
  expect(keyboard.getCascade()).toBe(17);
});
it('distinguishes focus before and after a value and caret location', () => {
  const input = document.createElement('input');
  const tracker = trackFocusSequence(input);
  expect([tracker.getHadFocus(), tracker.getFocusBeforeValue(), tracker.getCaretAtEnd()]).toEqual([
    false,
    false,
    null,
  ]);
  input.value = 'person';
  input.dispatchEvent(new Event('focusin'));
  expect(tracker.getHadFocus()).toBe(true);
  expect(tracker.getFocusBeforeValue()).toBe(false);
  input.setSelectionRange(0, 0);
  expect(tracker.getCaretAtEnd()).toBe(false);
  input.setSelectionRange(6, 6);
  expect(tracker.getCaretAtEnd()).toBe(true);
  input.value = '';
  input.dispatchEvent(new Event('focusin'));
  expect(tracker.getFocusBeforeValue()).toBe(true);
  tracker.destroy();
});
it('tracks scroll speed variance and bounds its history to 30 samples', () => {
  let now = 0;
  vi.spyOn(performance, 'now').mockImplementation(() => now);
  vi.spyOn(window, 'scrollY', 'get').mockImplementation(() => (now === 10 ? 1000 : now));
  const tracker = trackScroll();
  expect(tracker.getSpeedVariance()).toBe(0);
  window.dispatchEvent(new Event('scroll'));
  for (let i = 1; i <= 35; i++) {
    now = i * 10;
    window.dispatchEvent(new Event('scroll'));
  }
  expect(tracker.getCount()).toBe(36);
  expect(tracker.getSpeedVariance()).toBe(0);
  tracker.destroy();
  window.dispatchEvent(new Event('scroll'));
  expect(tracker.getCount()).toBe(36);
});
it.each([true, false])(
  'resolves visibility after its bounded observation (visible=%s)',
  async (visible) => {
    let observerCallback: IntersectionObserverCallback | undefined;
    const observe = vi.fn();
    const disconnect = vi.fn();
    class Observer {
      constructor(callback: IntersectionObserverCallback, options: IntersectionObserverInit) {
        observerCallback = callback;
        expect(options).toEqual({ threshold: 0.1 });
      }
      observe = observe;
      disconnect = disconnect;
    }
    vi.stubGlobal('IntersectionObserver', Observer);
    const button = document.createElement('button');
    const tracker = trackVisibility(button);
    expect(observe).toHaveBeenCalledExactlyOnceWith(button);
    expect(tracker.getVisible()).toBeNull();
    observerCallback?.(
      [{ isIntersecting: visible } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    );
    await vi.advanceTimersByTimeAsync(2000);
    expect(tracker.getVisible()).toBe(visible);
    expect(disconnect).toHaveBeenCalledTimes(1);
    tracker.destroy();
    expect(vi.getTimerCount()).toBe(0);
  },
);
it('collects initial aggregate signals and detaches the observer without browser visibility support', () => {
  vi.stubGlobal('IntersectionObserver', undefined);
  const input = document.createElement('input');
  const button = document.createElement('button');
  const observer = attachBehavioralObserver(input, button);
  const initial = observer.collect();
  expect(initial).toEqual({
    mouse_event_count: 0,
    mouse_variance: 0,
    mouse_path_length: 0,
    kb_dwell_variance: null,
    kb_dwell_mean: null,
    kb_flight_variance: null,
    kb_flight_mean: null,
    kb_sample_count: 0,
    event_cascade: 0,
    event_order_valid: null,
    keyboard_submit: false,
    email_had_focus: false,
    email_focus_before_value: false,
    caret_at_end: null,
    scroll_event_count: 0,
    scroll_speed_variance: 0,
    submit_visible: null,
  });
  observer.destroy();
  button.dispatchEvent(new Event('click'));
  mouse(100, 20);
  expect(observer.collect()).toEqual(initial);
});
