import { createHttpClient, type HttpClient } from '@nvbes/http-client';
import {
  AccountMeSchema,
  AccountsResponseSchema,
  AddSecondaryEmailResponseSchema,
  CreateWorkspaceResponseSchema,
  EmailAddressesResponseSchema,
  EmptySchema,
  ForgotPasswordResultSchema,
  GpcStatusSchema,
  OAuthClientsResponseSchema,
  PromoteSecondaryEmailResponseSchema,
  ResendSecondaryEmailVerificationResponseSchema,
  ResetPasswordResultSchema,
  SessionsResponseSchema,
  SuccessSchema,
  UserConsentSchema,
  WorkspacesResponseSchema,
  type AccountEntry,
  type AccountMe,
  type AccountSession,
  type AccountWorkspace,
  type AddSecondaryEmailResponse,
  type CreateWorkspaceInput,
  type EmailAddressesResponse,
  type ForgotPasswordResult,
  type GpcStatus,
  type OAuthClient,
  type OAuthClientsResponse,
  type PromoteSecondaryEmailResponse,
  type ResendSecondaryEmailVerificationResponse,
  type ResetPasswordResult,
  type UserConsent,
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

  addSecondaryEmail(email: string): Promise<AddSecondaryEmailResponse> {
    return this.http.post('/auth/me/emails', AddSecondaryEmailResponseSchema, { email });
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

  revokeSession(sessionId: string): Promise<void> {
    return this.http.delete(`/auth/sessions/${sessionId}`, SuccessSchema).then(() => undefined);
  }

  forgetAccount(authuser: string): Promise<void> {
    return this.http
      .delete(`/auth/accounts/${encodeURIComponent(authuser)}`, SuccessSchema)
      .then(() => undefined);
  }

  revokeOtherSessions(): Promise<void> {
    return this.http.post('/auth/sessions/revoke-others', SuccessSchema, {}).then(() => undefined);
  }

  logout(): Promise<void> {
    return this.http.post('/auth/logout', SuccessSchema, {}).then(() => undefined);
  }

  listConsents(options?: { signal?: AbortSignal }): Promise<UserConsent[]> {
    return this.http.get('/legal/consents', UserConsentSchema.array(), options);
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
    return this.http.delete(`/oauth/clients/${clientId}`, SuccessSchema).then(() => undefined);
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
