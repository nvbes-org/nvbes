import type { BeforeSendFn } from 'posthog-js';

const POSTHOG_URL_PROPERTIES = [
  '$current_url',
  '$initial_current_url',
  '$session_entry_url',
  '$referrer',
  '$initial_referrer',
  '$external_click_url',
  'url.full',
] as const;

export const stripPostHogUrlSecrets: BeforeSendFn = (event) => {
  if (!event?.properties) {
    return event;
  }

  const properties = { ...event.properties };
  let changed = false;
  for (const property of POSTHOG_URL_PROPERTIES) {
    const value = properties[property];
    if (typeof value !== 'string') {
      continue;
    }

    const sanitized = urlWithoutSearchOrHash(value);
    if (sanitized !== value) {
      properties[property] = sanitized;
      changed = true;
    }
  }

  if (!changed) {
    return event;
  }

  return {
    ...event,
    properties,
  };
};

function urlWithoutSearchOrHash(value: string): string {
  try {
    const url = new URL(value);
    url.search = '';
    url.hash = '';
    url.username = '';
    url.password = '';
    return url.href;
  } catch {
    return value.split(/[?#]/, 1)[0] ?? '';
  }
}
