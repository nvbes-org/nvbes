import { beforeEach, describe, expect, it, vi } from 'vitest';

const { post } = vi.hoisted(() => ({
  post: vi.fn(),
}));

vi.mock('./identity.http', () => ({
  identityHttpClient: { post },
}));

import { changeVerificationEmail } from './identity.email-verification';

describe('changeVerificationEmail', () => {
  beforeEach(() => {
    post.mockReset();
    post.mockResolvedValue({
      success: true,
      email_verified: false,
      verification_resend_available_at: null,
    });
  });

  it('never sends the current email as an enrollment credential', async () => {
    await changeVerificationEmail('corrected@example.com');

    expect(post).toHaveBeenCalledOnce();
    expect(post.mock.calls[0]?.[0]).toBe('/auth/verify-email/change');
    expect(post.mock.calls[0]?.[2]).toEqual({
      email: 'corrected@example.com',
      timezone: 'Europe/Paris',
    });
    expect(post.mock.calls[0]?.[2]).not.toHaveProperty('current_email');
  });
});
