export interface AccountProfile {
  id: string;
  displayName: string;
  firstname: string | null;
  lastname: string | null;
  username: string | null;
  region: string | null;
}

export function parseAccountProfile(value: unknown, subject: string): AccountProfile {
  if (!record(value) || !record(value.user)) throw new Error('Invalid Account profile');
  const user = value.user;
  if (user.id !== subject || typeof user.display_name !== 'string')
    throw new Error('Account profile identity mismatch');
  const optional = (key: string): string | null => {
    const field = user[key];
    if (field !== null && typeof field !== 'string') throw new Error('Invalid profile field');
    return field;
  };
  return {
    id: subject,
    displayName: user.display_name,
    firstname: optional('firstname'),
    lastname: optional('lastname'),
    username: optional('username'),
    region: optional('region'),
  };
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
