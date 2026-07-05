import { createDecoyLinks } from '@nvbes/identity-sdk-web';
import type { LoginPageDecoyRef } from './useLoginPage.bootstrap.types';

export function mountLoginDecoyLinks(decoyRef: LoginPageDecoyRef, containerId: string) {
  const container = document.getElementById(containerId);
  if (!container) {
    return undefined;
  }

  decoyRef.current = createDecoyLinks(container, 3);
  return () => decoyRef.current?.destroy();
}
