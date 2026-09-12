import { describe, expect, it } from 'vite-plus/test';
import {
  createErrorReportingReplayPrivacyOptions,
  sanitizeUrlString,
  scrubErrorReportingEvent,
  scrubReplayRecordingEvent,
} from './error-reporting-privacy';

describe('error reporting privacy boundaries', () => {
  it('configures replay without body, media, header or text collection', () => {
    expect(createErrorReportingReplayPrivacyOptions()).toEqual({
      maskAllText: true,
      maskAllInputs: true,
      blockAllMedia: true,
      networkDetailAllowUrls: [],
      networkDetailDenyUrls: [/.*/],
      networkCaptureBodies: false,
      networkRequestHeaders: [],
      networkResponseHeaders: [],
      beforeAddRecordingEvent: scrubReplayRecordingEvent,
    });
  });

  it.each([null, 42, false, undefined])('preserves non-record values %j', (value) => {
    expect(scrubErrorReportingEvent(value)).toBe(value);
    expect(scrubReplayRecordingEvent(value)).toBe(value);
  });

  it.each([
    { data: null },
    { data: {} },
    { data: { payload: [] } },
    { data: { payload: { type: 'network', category: 'fetch' } } },
  ])('preserves non-console frames: %j', (value) => {
    expect(scrubReplayRecordingEvent(value)).toEqual(value);
  });

  it.each([{ category: 'console' }, { type: 'console' }])('drops console payload %j', (payload) => {
    expect(scrubReplayRecordingEvent({ data: { payload } })).toBeNull();
  });

  it('scrubs nested arrays and sensitive assignments without mutating the input', () => {
    const input = {
      data: ['user@example.test', 'token=synthetic', { authorization: 42, ok: true }],
      request: {
        headers: {
          Accept: 'application/json',
          'Content-Length': 12,
          'Content-Type': 'text/plain',
          'X-Private': 'secret',
        },
        env: null,
      },
    };
    const original = structuredClone(input);
    expect(scrubErrorReportingEvent(input)).toEqual({
      data: ['[Filtered]', 'token=[Filtered]', { authorization: '[Filtered]', ok: true }],
      request: { headers: { Accept: 'application/json', 'Content-Type': 'text/plain' }, env: {} },
    });
    expect(input).toEqual(original);
  });

  it.each([
    ['/assets/style.css', '/assets/style.css'],
    ['/files/report.csv', '/files/[id]'],
    ['/users/12345678-1234-4123-8123-123456789abc', '/users/[id]'],
    ['/tokens/abcdefghijklmnopqrstuvwxyz', '/tokens/[id]'],
    ['/bad/%zz', '/bad/%zz'],
    [
      'https://user:pass@example.test/path?q=secret#fragment',
      'https://example.test/path?q=%5BFiltered%5D',
    ],
    ['http://[broken token=synthetic', 'http://[broken token=[Filtered]'],
  ])('sanitizes URL %j', (input, expected) => {
    expect(sanitizeUrlString(input)).toBe(expected);
  });
});
