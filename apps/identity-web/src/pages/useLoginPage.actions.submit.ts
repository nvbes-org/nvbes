import { buildIdentifierSubmitAction } from './useLoginPage.actions.submit.identifier';
import {
  buildMfaSubmitAction,
  buildResetToIdentifierAction,
} from './useLoginPage.actions.submit.mfa';
import { buildPasswordSubmitAction } from './useLoginPage.actions.submit.password';
import type { SubmitActionOptions } from './useLoginPage.actions.submit.shared';

export function useLoginPageSubmitActions(
  options: SubmitActionOptions,
  finishLogin: (session: string | null) => Promise<void>,
) {
  return {
    handleIdentifierSubmit: buildIdentifierSubmitAction(options),
    handlePasswordSubmit: buildPasswordSubmitAction(options, finishLogin),
    handleMfaSubmit: buildMfaSubmitAction(options, finishLogin),
    resetToIdentifier: buildResetToIdentifierAction(options),
  };
}
