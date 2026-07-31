export const accountQueryKeys = {
  all: ['account'] as const,
  profile: ['account', 'profile'] as const,
  preferences: ['account', 'preferences'] as const,
  notifications: ['account', 'notifications'] as const,
  consents: ['account', 'privacy', 'consents'] as const,
  gpc: ['account', 'privacy', 'gpc'] as const,
};
