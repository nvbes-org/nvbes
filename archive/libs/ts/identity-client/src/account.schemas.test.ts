import { expect, it } from 'vite-plus/test';
import {
  AccountEntrySchema,
  AccountSessionClientSchema,
  CreateWorkspaceInputSchema,
  SuccessSchema,
} from './account.schemas';

it.each(['console', 'desktop', 'mobile', 'tablet', 'unknown', 'wearable'])(
  'preserves the valid %s device category and client metadata',
  (device_type) => {
    const client = {
      browser: 'Firefox',
      browser_version: 142,
      os: 'Linux',
      os_version: null,
      device: 'Test',
      device_type,
    };
    expect(AccountSessionClientSchema.parse(client)).toEqual(client);
  },
);
it('rejects malformed client metadata and unknown device categories', () => {
  const client = {
    browser: null,
    browser_version: null,
    os: null,
    os_version: null,
    device: null,
    device_type: 'unknown',
  };
  expect(AccountSessionClientSchema.parse(client)).toEqual(client);
  expect(AccountSessionClientSchema.safeParse({ ...client, device_type: 'invalid' }).success).toBe(
    false,
  );
  expect(AccountSessionClientSchema.safeParse({ ...client, browser_version: '142' }).success).toBe(
    false,
  );
  expect(AccountSessionClientSchema.safeParse({}).success).toBe(false);
});
it.each(['active', 'expired'])('accepts and preserves %s account status', (status) => {
  const schema = AccountEntrySchema.pick({ status: true });
  expect(schema.parse({ status })).toEqual({ status });
  expect(schema.safeParse({ status: 'invalid' }).success).toBe(false);
});
it.each([true, false])('preserves success=%s and rejects malformed status responses', (success) => {
  expect(SuccessSchema.parse({ success })).toEqual({ success });
  expect(SuccessSchema.safeParse({ success: 'true' }).success).toBe(false);
  expect(SuccessSchema.safeParse({}).success).toBe(false);
});
it.each([1, 2, 99, 100])(
  'accepts a workspace name of %s characters without truncation',
  (length) => {
    const input = { name: 'a'.repeat(length), workspace_type: 'team' };
    expect(CreateWorkspaceInputSchema.parse(input)).toEqual(input);
  },
);
it.each(['', 'a'.repeat(101)])(
  'rejects workspace names outside the 1–100 character contract',
  (name) => {
    expect(CreateWorkspaceInputSchema.safeParse({ name }).success).toBe(false);
  },
);
