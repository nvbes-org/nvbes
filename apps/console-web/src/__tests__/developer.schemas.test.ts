import { describe, expect, test } from 'vite-plus/test';
import { DeveloperOAuthClientsSchema, DeveloperScopeRegistrySchema } from '../developer.schemas';

describe('developer schemas', () => {
  test('parses OAuth client summaries with marketplace and consent status', () => {
    const result = DeveloperOAuthClientsSchema.parse({
      oauth_clients: [
        {
          client_id: 'client_123',
          name: 'Drive Sync',
          status: 'active',
          marketplace_status: 'approved',
          consent_screen_configured: true,
          redirect_uri_count: 2,
          allowed_scopes: ['openid', 'drive.files.read'],
          health_status: 'passing',
        },
      ],
    });

    expect(result.oauth_clients[0].marketplace_status).toBe('approved');
  });

  test('parses scope registry risk and lifecycle metadata', () => {
    const result = DeveloperScopeRegistrySchema.parse({
      scopes: [
        {
          scope_key: 'drive.files.read',
          display_name: 'Read files',
          description: 'Read Drive file metadata and content.',
          risk: 'medium',
          owner_team: 'drive',
          lifecycle: 'active',
          allowed_audiences: ['cloud-service'],
        },
      ],
    });

    expect(result.scopes[0].risk).toBe('medium');
  });
});
