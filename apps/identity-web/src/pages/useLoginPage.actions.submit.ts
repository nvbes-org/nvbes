import { useRef, type RefObject } from 'react';
import { buildIdentifierSubmitAction } from './useLoginPage.actions.submit.identifier';
import {
  buildMfaSubmitAction,
  buildResetToIdentifierAction,
} from './useLoginPage.actions.submit.mfa';
import { buildPasswordSubmitAction } from './useLoginPage.actions.submit.password';
import type { LoginFormHandler, SubmitActionOptions } from './useLoginPage.actions.submit.shared';

function guardedSubmit(lock: RefObject<boolean>, handler: LoginFormHandler): LoginFormHandler {
  return async (event) => {
    if (lock.current) {
      event.preventDefault();
      return;
    }

    lock.current = true;
    try {
      await handler(event);
    } finally {
      lock.current = false;
    }
  };
}

export function useLoginPageSubmitActions(
  options: SubmitActionOptions,
  finishLogin: (session: string | null) => Promise<void>,
) {
  const identifierLock = useRef(false);
  const passwordLock = useRef(false);
  const mfaLock = useRef(false);

  return {
    handleIdentifierSubmit: guardedSubmit(identifierLock, buildIdentifierSubmitAction(options)),
    handlePasswordSubmit: guardedSubmit(
      passwordLock,
      buildPasswordSubmitAction(options, finishLogin),
    ),
    handleMfaSubmit: guardedSubmit(mfaLock, buildMfaSubmitAction(options, finishLogin)),
    resetToIdentifier: buildResetToIdentifierAction(options),
  };
}
