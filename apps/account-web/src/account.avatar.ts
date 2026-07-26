const PROFILE_AVATAR_UPDATED = 'nvbes:profile-avatar-updated';

export function profileAvatarUrl(authuser: string, version: number): string {
  const query = authuser === '0' ? '' : `?authuser=${encodeURIComponent(authuser)}`;
  return `/auth/me/avatar${query}${query ? '&' : '?'}v=${version}`;
}

export function notifyProfileAvatarUpdated(): void {
  window.dispatchEvent(new CustomEvent(PROFILE_AVATAR_UPDATED));
}

export function subscribeToProfileAvatarUpdates(listener: () => void): () => void {
  window.addEventListener(PROFILE_AVATAR_UPDATED, listener);
  return () => window.removeEventListener(PROFILE_AVATAR_UPDATED, listener);
}
