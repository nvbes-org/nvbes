import type { HostedLoginDecision } from '../identity.universal-login.api';
import {
  approveHostedConsent,
  authorizeHostedLogin,
  denyHostedConsent,
  getHostedLoginDecision,
} from '../identity.universal-login.api';

export function shouldShowHostedConsent(decision: HostedLoginDecision | null): boolean {
  return decision?.kind === 'consent_required';
}

export function followHostedDecision(decision: HostedLoginDecision): void {
  if (decision.kind === 'redirect') {
    window.location.assign(decision.redirect_url);
  }
}

export async function loadHostedLogin(stateId: string): Promise<HostedLoginDecision> {
  return getHostedLoginDecision(stateId);
}

export async function resumeHostedAuthorization(stateId: string): Promise<HostedLoginDecision> {
  return authorizeHostedLogin(stateId);
}

export async function approveUniversalLoginConsent(
  stateId: string,
): Promise<HostedLoginDecision> {
  return approveHostedConsent(stateId);
}

export async function denyUniversalLoginConsent(stateId: string): Promise<HostedLoginDecision> {
  return denyHostedConsent(stateId);
}
