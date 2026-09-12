import { expect, it, vi } from 'vite-plus/test';
import { z } from 'zod';
import {
  DtoValidationError,
  HttpError,
  readErrorMessage,
  readRequestId,
  readResponseBody,
} from './http.errors';

it('preserves typed error identity and the original validation cause', () => {
  const cause = new z.ZodError([]);
  const dto = new DtoValidationError(cause);
  expect(dto.name).toBe('DtoValidationError');
  expect(dto.message).toBe('Response DTO validation failed');
  expect(dto.cause).toBe(cause);
  const http = new HttpError('failed', new Response(null, { status: 401 }), {});
  expect(http.name).toBe('HttpError');
  expect(http.message).toBe('failed');
  expect(http.recovery).toBeUndefined();
});

it('does not consume a 204 body stream', async () => {
  const response = new Response(null, { status: 204 });
  const read = vi.spyOn(response, 'text');
  expect(await readResponseBody(response)).toBeUndefined();
  expect(read).not.toHaveBeenCalled();
});

it('handles valid envelopes with no optional error object', () => {
  const response = new Response(null, {
    status: 400,
    statusText: 'Bad Request',
    headers: { 'x-request-id': 'request-1' },
  });
  expect(readErrorMessage(response, {})).toBe('400 Bad Request');
  expect(readRequestId(response, {})).toBe('request-1');
  expect(new HttpError('failed', response, {}).recovery).toBeUndefined();
});
