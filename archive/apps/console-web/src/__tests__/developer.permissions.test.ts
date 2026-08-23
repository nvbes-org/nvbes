import { describe, expect, it } from 'vite-plus/test';
import { DEVELOPER_PERMISSIONS, canUseDeveloperPermission } from '../developer.permissions';

describe('developer permissions', () => {
  it('allows permissions explicitly returned by the API', () => {
    expect(
      canUseDeveloperPermission(
        {
          roles: ['app_manager'],
          permissions: ['apps.read', 'secrets.rotate'],
        },
        'secrets.rotate',
      ),
    ).toBe(true);
  });

  it('denies permissions that are not returned by the API', () => {
    expect(
      canUseDeveloperPermission(
        {
          roles: ['log_viewer'],
          permissions: ['logs.read'],
        },
        'webhooks.replay',
      ),
    ).toBe(false);
  });

  it('keeps the admin role aligned with the known permission registry', () => {
    for (const permission of DEVELOPER_PERMISSIONS) {
      expect(
        canUseDeveloperPermission(
          {
            roles: ['developer_admin'],
            permissions: [],
          },
          permission,
        ),
      ).toBe(true);
    }
  });
});
