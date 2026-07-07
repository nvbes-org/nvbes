import { z } from 'zod';

const STORAGE_KEY = 'nvbes.identity.emailVerification';

const VerificationSnapshotSchema = z.object({
  accountName: z.string().nullable().optional(),
  email: z.string(),
  resendAvailableAt: z.string().nullable(),
});

export type VerificationSnapshot = z.infer<typeof VerificationSnapshotSchema>;

function verificationStorage(): Storage | null {
  try {
    return globalThis.sessionStorage ?? null;
  } catch {
    return null;
  }
}

export function readVerificationSnapshot(): VerificationSnapshot | null {
  const storage = verificationStorage();
  if (!storage) {
    return null;
  }

  const raw = storage.getItem(STORAGE_KEY);
  if (!raw) {
    return null;
  }

  try {
    return VerificationSnapshotSchema.parse(JSON.parse(raw));
  } catch {
    storage.removeItem(STORAGE_KEY);
    return null;
  }
}

export function saveVerificationSnapshot(snapshot: VerificationSnapshot): void {
  const storage = verificationStorage();
  if (!storage || snapshot.email.trim().length === 0) {
    return;
  }

  storage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      accountName: snapshot.accountName,
      email: snapshot.email,
      resendAvailableAt: snapshot.resendAvailableAt,
    }),
  );
}

export function clearVerificationSnapshot(): void {
  verificationStorage()?.removeItem(STORAGE_KEY);
}
