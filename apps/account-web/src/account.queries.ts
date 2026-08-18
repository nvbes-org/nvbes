export const accountQueryKeys = {
  all: ['account'] as const,
  profile: ['account', 'profile'] as const,
  preferences: ['account', 'preferences'] as const,
  notifications: ['account', 'notifications'] as const,
  consents: ['account', 'privacy', 'consents'] as const,
  gpc: ['account', 'privacy', 'gpc'] as const,
  export: ['account', 'privacy', 'export'] as const,
  closure: ['account', 'privacy', 'closure'] as const,
  sessions: ['account', 'security', 'sessions'] as const,
};
