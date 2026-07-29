import { createHttpClient, type HttpClient } from '@nvbes/http-client';
import { z } from 'zod';
import {
  type AccountEntry,
  type AccountMe,
  AccountMeSchema,
  type AccountSession,
  AccountSessionSchema,
  AccountsResponseSchema,
  type AccountWorkspace,
  type AddSecondaryEmailResponse,
  AddSecondaryEmailResponseSchema,
  ConsentHistoryResponseSchema,
  type ConsentHistoryResponse,
  type CreateWorkspaceInput,
  CreateWorkspaceResponseSchema,
  type DeviceTrustResult,
  DeviceTrustResultSchema,
  type EmailAddressesResponse,
  EmailAddressesResponseSchema,
  EmptySchema,
  type ForgotPasswordResult,
  ForgotPasswordResultSchema,
  type GpcStatus,
  GpcStatusSchema,
  type OAuthClient,
  type OAuthClientsResponse,
  OAuthClientsResponseSchema,
  type PromoteSecondaryEmailResponse,
  PromoteSecondaryEmailResponseSchema,
  type ResendSecondaryEmailVerificationResponse,
  ResendSecondaryEmailVerificationResponseSchema,
  type ResetPasswordResult,
  ResetPasswordResultSchema,
  SessionsResponseSchema,
  SuccessSchema,
  type UserConsent,
  UserConsentSchema,
  WorkspacesResponseSchema,
} from './account.schemas';

export type RequestOptions = { signal?: AbortSignal };

export type IdentityClientOptions = {
  baseUrl?: string;
  http?: HttpClient;
};

export class AccountIdentityClient {
  protected readonly http: HttpClient;

  constructor(options: IdentityClientOptions = {}) {
    this.http =
      options.http ??
      createHttpClient({
        baseUrl: options.baseUrl,
        credentials: 'include',
      });
  }

  getMe(options?: { signal?: AbortSignal }): Promise<AccountMe> {
    return this.http.get<AccountMe>('/auth/me', AccountMeSchema, options);
  }

  listEmails(options?: RequestOptions): Promise<EmailAddressesResponse> {
    return this.http.get('/auth/me/emails', EmailAddressesResponseSchema, options);
  }

  listEmailsPage(
    options: { limit?: number; cursor?: string; signal?: AbortSignal } = {},
  ): Promise<EmailAddressesResponse> {
    const params = new URLSearchParams();
    if (options.limit !== undefined) params.set('limit', String(options.limit));
    if (options.cursor) params.set('cursor', options.cursor);
    const query = params.toString();
    return this.http.get(
      `/auth/me/emails${query ? `?${query}` : ''}`,
      EmailAddressesResponseSchema,
      { signal: options.signal },
    );
  }

  addSecondaryEmail(email: string): Promise<AddSecondaryEmailResponse> {
    return this.http.post('/auth/me/emails', AddSecondaryEmailResponseSchema, {
      email,
    });
  }

  promoteSecondaryEmail(emailId: string): Promise<PromoteSecondaryEmailResponse> {
    return this.http.post(
      `/auth/me/emails/${encodeURIComponent(emailId)}/promote`,
      PromoteSecondaryEmailResponseSchema,
      {},
    );
  }

  resendSecondaryEmailVerification(
    emailId: string,
  ): Promise<ResendSecondaryEmailVerificationResponse> {
    return this.http.post(
      `/auth/me/emails/${encodeURIComponent(emailId)}/resend-verification`,
      ResendSecondaryEmailVerificationResponseSchema,
      {},
    );
  }

  deleteSecondaryEmail(emailId: string): Promise<void> {
    return this.http
      .delete(`/auth/me/emails/${encodeURIComponent(emailId)}`, SuccessSchema)
      .then(() => undefined);
  }

  listWorkspaces(options?: { signal?: AbortSignal }): Promise<AccountWorkspace[]> {
    return this.http
      .get('/workspaces', WorkspacesResponseSchema, options)
      .then((response) => response.workspaces ?? []);
  }

  listAccounts(options?: { signal?: AbortSignal }): Promise<AccountEntry[]> {
    return this.http
      .get('/auth/accounts', AccountsResponseSchema, options)
      .then((response) => response.accounts ?? []);
  }

  listSessions(options?: { signal?: AbortSignal }): Promise<AccountSession[]> {
    return this.http
      .get('/auth/sessions', SessionsResponseSchema, options)
      .then((response) => response.sessions);
  }

  listSessionsPage(
    options: { limit?: number; cursor?: string; signal?: AbortSignal } = {},
  ): Promise<z.infer<typeof SessionsResponseSchema>> {
    const params = new URLSearchParams();
    if (options.limit !== undefined) params.set('limit', String(options.limit));
    if (options.cursor) params.set('cursor', options.cursor);
    const query = params.toString();
    return this.http.get(`/auth/sessions${query ? `?${query}` : ''}`, SessionsResponseSchema, {
      signal: options.signal,
    });
  }

  revokeSession(sessionId: string): Promise<void> {
    return this.http
      .delete(`/auth/sessions/${encodeURIComponent(sessionId)}`, SuccessSchema)
      .then(() => undefined);
  }

  confirmHighRiskSession(sessionId: string): Promise<AccountSession> {
    return this.http.post(
      `/auth/sessions/${encodeURIComponent(sessionId)}/confirm`,
      AccountSessionSchema,
      {},
    );
  }

  forgetAccount(authuser: string): Promise<void> {
    return this.http
      .delete(`/auth/accounts/${encodeURIComponent(authuser)}`, SuccessSchema)
      .then(() => undefined);
  }

  revokeOtherSessions(): Promise<void> {
    return this.http.post('/auth/sessions/revoke-others', SuccessSchema, {}).then(() => undefined);
  }

  revokeAllSessions(): Promise<void> {
    return this.http.post('/auth/sessions/revoke-all', SuccessSchema, {}).then(() => undefined);
  }

  trustDevice(deviceId: string): Promise<DeviceTrustResult> {
    return this.http.post(
      `/auth/devices/${encodeURIComponent(deviceId)}/trust`,
      DeviceTrustResultSchema,
      {},
    );
  }

  revokeDevice(deviceId: string): Promise<DeviceTrustResult> {
    return this.http.delete(
      `/auth/devices/${encodeURIComponent(deviceId)}`,
      DeviceTrustResultSchema,
    );
  }

  logout(): Promise<void> {
    return this.http.post('/auth/logout', SuccessSchema, {}).then(() => undefined);
  }

  listConsents(options?: { signal?: AbortSignal }): Promise<UserConsent[]> {
    return this.http
      .get('/legal/consents', ConsentHistoryResponseSchema, options)
      .then((response) => response.consents);
  }

  listConsentsPage(
    options: { limit?: number; cursor?: string; signal?: AbortSignal } = {},
  ): Promise<ConsentHistoryResponse> {
    const params = new URLSearchParams();
    if (options.limit !== undefined) params.set('limit', String(options.limit));
    if (options.cursor) params.set('cursor', options.cursor);
    const query = params.toString();
    return this.http.get(
      `/legal/consents${query ? `?${query}` : ''}`,
      ConsentHistoryResponseSchema,
      { signal: options.signal },
    );
  }

  grantConsent(consentType: string, documentVersion: string): Promise<UserConsent> {
    return this.http.post('/legal/consent', UserConsentSchema, {
      consent_type: consentType,
      document_version: documentVersion,
    });
  }

  gpcStatus(options?: { signal?: AbortSignal }): Promise<GpcStatus> {
    return this.http.get('/legal/gpc', GpcStatusSchema, options);
  }

  revokeConsent(consentType: string, documentVersion: string): Promise<void> {
    return this.http
      .post('/legal/consent/revoke', EmptySchema, {
        consent_type: consentType,
        document_version: documentVersion,
      })
      .then(() => undefined);
  }

  listOAuthClients(options?: RequestOptions): Promise<OAuthClient[]> {
    return this.http
      .get<OAuthClientsResponse>('/oauth/clients', OAuthClientsResponseSchema, options)
      .then((response) => response.clients);
  }

  revokeOAuthClient(clientId: string): Promise<void> {
    return this.http
      .delete(`/oauth/clients/${encodeURIComponent(clientId)}`, SuccessSchema)
      .then(() => undefined);
  }

  createWorkspace(input: CreateWorkspaceInput): Promise<AccountWorkspace> {
    return this.http
      .post('/workspaces', CreateWorkspaceResponseSchema, input)
      .then((response) => response.workspace);
  }

  forgotPassword(email: string): Promise<ForgotPasswordResult> {
    return this.http.post('/auth/password/forgot', ForgotPasswordResultSchema, {
      email,
    });
  }

  resetPassword(token: string, newPassword: string): Promise<ResetPasswordResult> {
    return this.http.post('/auth/password/reset', ResetPasswordResultSchema, {
      token,
      new_password: newPassword,
    });
  }
}
