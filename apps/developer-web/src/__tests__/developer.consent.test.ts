import { describe, expect, it } from 'vite-plus/test';
import { DeveloperConsentScreenSchema } from '../developer.schemas';

describe('developer consent screen schema', () => {
  it('parses configured consent screens', () => {
    const parsed = DeveloperConsentScreenSchema.parse({
      client_id: 'client-1',
      product_name: 'Drive Partner',
      logo_url: null,
      support_url: 'https://support.example.test',
      privacy_url: 'https://privacy.example.test',
      terms_url: null,
      description: 'Access files and account profile.',
      configured: true,
      updated_at: '2026-06-14T10:00:00Z',
    });

    expect(parsed.configured).toBe(true);
    expect(parsed.privacy_url).toBe('https://privacy.example.test');
  });
});
