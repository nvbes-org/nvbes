interface PasswordCredential extends Credential {
  readonly id: string;
  readonly password: string;
  readonly name?: string;
  readonly iconURL?: string;
  readonly type: 'password';
}

declare global {
  interface PasswordCredentialData {
    id: string;
    password: string;
    name?: string;
    iconURL?: string;
  }

  interface FederatedCredentialData {
    id: string;
    provider: string;
    name?: string;
    iconURL?: string;
  }

  interface CredentialCreationOptions {
    password?: PasswordCredentialData;
    federated?: FederatedCredentialData;
  }

  interface CredentialRequestOptions {
    mediation?: CredentialMediationRequirement;
    password?: boolean;
  }
}

const PasswordCredentialCtor =
  typeof window !== 'undefined'
    ? ((
        window as unknown as {
          PasswordCredential?: new (data: PasswordCredentialData) => PasswordCredential;
        }
      ).PasswordCredential as
        | (new (data: PasswordCredentialData) => PasswordCredential)
        | undefined)
    : undefined;

export function isCredentialManagementSupported(): boolean {
  return (
    typeof window !== 'undefined' && 'PasswordCredential' in window && 'credentials' in navigator
  );
}

export async function getStoredPasswordCredential(): Promise<{
  id: string;
  password: string;
  name?: string;
} | null> {
  if (!isCredentialManagementSupported()) {
    return null;
  }
  try {
    const credential = await navigator.credentials.get({
      password: true,
      mediation: 'silent',
    } as CredentialRequestOptions);
    if (credential && credential.type === 'password') {
      const pc = credential as PasswordCredential;
      return { id: pc.id, password: pc.password, name: pc.name };
    }
    return null;
  } catch {
    return null;
  }
}

export async function storePasswordCredential(
  id: string,
  password: string,
  name?: string,
): Promise<void> {
  if (!isCredentialManagementSupported() || !PasswordCredentialCtor) {
    return;
  }
  try {
    const cred = new PasswordCredentialCtor({ id, password, name });
    await navigator.credentials.store(cred);
  } catch {
    // Le navigateur peut refuser de stocker (utilisateur annule, etc.)
  }
}

export async function preventAutoSignIn(): Promise<void> {
  if (typeof navigator !== 'undefined' && 'credentials' in navigator) {
    try {
      await navigator.credentials.preventSilentAccess();
    } catch {
      // Best-effort
    }
  }
}
