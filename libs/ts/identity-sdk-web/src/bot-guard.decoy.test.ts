// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { createDecoyField, createDecoyLinks, mountDecoyField } from './bot-guard.decoy';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

it('creates an inaccessible autofill-resistant field and releases every owned resource', () => {
  const disconnect = vi.spyOn(MutationObserver.prototype, 'disconnect');
  const before = document.head.querySelectorAll('style').length;
  const field = createDecoyField();
  document.body.append(field.wrapper);
  const input = field.wrapper.querySelector('input');
  expect(input).not.toBeNull();
  expect(input?.type).toBe('text');
  expect(input?.hasAttribute('name')).toBe(false);
  expect(input?.readOnly).toBe(true);
  expect(input?.autocomplete).toBe('new-password');
  expect(input?.tabIndex).toBe(-1);
  expect(input?.getAttribute('aria-hidden')).toBe('true');
  expect(field.wrapper.getAttribute('aria-hidden')).toBe('true');
  expect(field.wrapper.querySelector('label')?.htmlFor).toBe(input?.id);
  expect(document.head.querySelectorAll('style')).toHaveLength(before + 1);
  expect(field.getValue()).toBe('');
  field.destroy();
  field.destroy();
  expect(disconnect).toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
  expect(document.head.querySelectorAll('style')).toHaveLength(before);
});
it.each(['input', 'paste'])('clears autofill on %s and removes the handler on destroy', (type) => {
  const field = createDecoyField();
  document.body.append(field.wrapper);
  const input = field.wrapper.querySelector('input');
  if (!input) throw new Error('Missing decoy');
  input.value = 'autofilled';
  const event = new Event(type, { cancelable: true });
  input.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  expect(field.getValue()).toBe('');
  field.destroy();
  input.value = 'after-destroy';
  input.dispatchEvent(new Event(type));
  expect(field.getValue()).toBe('after-destroy');
});
it('clears direct property writes at 200ms, not before', async () => {
  const field = createDecoyField();
  document.body.append(field.wrapper);
  const input = field.wrapper.querySelector('input');
  if (!input) throw new Error('Missing decoy');
  input.value = 'injected';
  await vi.advanceTimersByTimeAsync(199);
  expect(field.getValue()).toBe('injected');
  await vi.advanceTimersByTimeAsync(1);
  expect(field.getValue()).toBe('');
  await vi.advanceTimersByTimeAsync(200);
  expect(field.getValue()).toBe('');
  field.destroy();
});
it('reacts only to its own autofill animation', () => {
  const field = createDecoyField();
  document.body.append(field.wrapper);
  const input = field.wrapper.querySelector('input');
  if (!input) throw new Error('Missing decoy');
  const style = [...document.head.querySelectorAll('style')].at(-1)?.textContent ?? '';
  const animationName = /@keyframes\s+(\w+)/.exec(style)?.[1];
  expect(animationName).toBeDefined();
  input.value = 'injected';
  for (const name of ['unrelated', animationName]) {
    const event = new Event('animationstart');
    Object.defineProperty(event, 'animationName', { value: name });
    input.dispatchEvent(event);
    expect(field.getValue()).toBe(name === 'unrelated' ? 'injected' : '');
  }
  field.destroy();
});
it('mounts after 80ms and cleans up after removal from the document', async () => {
  const container = document.createElement('form');
  document.body.append(container);
  const before = document.head.querySelectorAll('style').length;
  const pending = mountDecoyField(container);
  await vi.advanceTimersByTimeAsync(79);
  expect(container.children).toHaveLength(0);
  await vi.advanceTimersByTimeAsync(1);
  const field = await pending;
  expect(container.contains(field.wrapper)).toBe(true);
  field.wrapper.remove();
  await vi.advanceTimersByTimeAsync(1);
  expect(document.head.querySelectorAll('style')).toHaveLength(before);
  expect(vi.getTimerCount()).toBe(0);
});
it.each(['click', 'auxclick'])('tracks hidden link %s and removes handlers', (type) => {
  const container = document.createElement('div');
  const tracker = createDecoyLinks(container, 2);
  const links = [...container.querySelectorAll('a')];
  expect(links).toHaveLength(2);
  expect(tracker.wasClicked()).toBe(false);
  for (const link of links) {
    expect(link.tabIndex).toBe(-1);
    expect(link.getAttribute('aria-hidden')).toBe('true');
  }
  const event = new Event(type, { cancelable: true });
  links[0]?.dispatchEvent(event);
  expect(tracker.wasClicked()).toBe(true);
  expect(event.defaultPrevented).toBe(true);
  tracker.destroy();
  const after = new Event(type, { cancelable: true });
  links[1]?.dispatchEvent(after);
  expect(after.defaultPrevented).toBe(false);
});
