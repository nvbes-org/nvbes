import { getStoredPasswordCredential } from '@nvbes/identity-sdk-web';

export async function autoFillStoredPassword({
  setEmail,
  setPassword,
}: {
  setEmail: (value: string) => void;
  setPassword: (value: string) => void;
}) {
  const stored = await getStoredPasswordCredential();
  if (!stored) {
    return;
  }

  setEmail(stored.id);
  setPassword(stored.password);
}
