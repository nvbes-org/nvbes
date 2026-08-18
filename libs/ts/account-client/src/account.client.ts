import {
  type AccountAvatarUpload,
  type AccountAvatarUploadInput,
  AccountAvatarUploadSchema,
  type AccountConsent,
  type AccountConsentHistory,
  AccountConsentHistorySchema,
  type AccountConsentInput,
  AccountConsentSchema,
  type AccountClosure,
  AccountClosureSchema,
  type AccountClosureStatus,
  AccountClosureStatusSchema,
  type AccountGpcStatus,
  AccountGpcStatusSchema,
  type AccountExportRequest,
  AccountExportRequestSchema,
  type AccountExportStatus,
  AccountExportStatusSchema,
  type AccountNotifications,
  AccountNotificationsSchema,
  type AccountPreferences,
  AccountPreferencesSchema,
  type AccountProfile,
  AccountProfileEnvelopeSchema,
  type AccountUpdateProfileInput,
  EmptyResponseSchema,
  AccountSuccessSchema,
  type AccountSessionsPage,
  AccountSessionsPageSchema,
} from './account.schemas';
import {
  type AccountRequestOptions,
  AccountTransport,
  type AccountTransportOptions,
} from './account.transport';

export type AccountClientOptions = AccountTransportOptions;

export type AccountConsentPageOptions = AccountRequestOptions & {
  cursor?: string;
  limit?: number;
};

export type AccountSessionPageOptions = AccountRequestOptions & {
  cursor?: string;
  limit?: number;
};

export class AccountClient {
  private readonly transport: AccountTransport;

  constructor(options: AccountClientOptions) {
    this.transport = new AccountTransport(options);
  }

  getProfile(options?: AccountRequestOptions): Promise<AccountProfile> {
    return this.transport
      .request('/api/v1/profile', AccountProfileEnvelopeSchema, { ...options, method: 'GET' })
      .then(({ user }) => user);
  }

  updateProfile(
    input: AccountUpdateProfileInput,
    options?: AccountRequestOptions,
  ): Promise<AccountProfile> {
    return this.transport
      .request('/api/v1/profile', AccountProfileEnvelopeSchema, {
        ...options,
        body: input,
        method: 'PUT',
      })
      .then(({ user }) => user);
  }

  prepareAvatarUpload(
    input: AccountAvatarUploadInput,
    options?: AccountRequestOptions,
  ): Promise<AccountAvatarUpload> {
    return this.transport.request('/api/v1/profile/avatar', AccountAvatarUploadSchema, {
      ...options,
      body: input,
      method: 'POST',
    });
  }

  downloadAvatar(options?: AccountRequestOptions): Promise<Blob> {
    return this.transport.requestBlob('/api/v1/profile/avatar', options);
  }

  deleteAvatar(options?: AccountRequestOptions): Promise<void> {
    return this.success('/api/v1/profile/avatar', 'DELETE', options);
  }

  getPreferences(options?: AccountRequestOptions): Promise<AccountPreferences> {
    return this.transport.request('/api/v1/preferences', AccountPreferencesSchema, {
      ...options,
      method: 'GET',
    });
  }

  updatePreferences(
    preferences: AccountPreferences,
    options?: AccountRequestOptions,
  ): Promise<AccountPreferences> {
    return this.transport.request('/api/v1/preferences', AccountPreferencesSchema, {
      ...options,
      body: preferences,
      method: 'PUT',
    });
  }

  getNotifications(options?: AccountRequestOptions): Promise<AccountNotifications> {
    return this.transport.request('/api/v1/notifications', AccountNotificationsSchema, {
      ...options,
      method: 'GET',
    });
  }

  updateNotifications(
    notifications: AccountNotifications,
    options?: AccountRequestOptions,
  ): Promise<AccountNotifications> {
    return this.transport.request('/api/v1/notifications', AccountNotificationsSchema, {
      ...options,
      body: notifications,
      method: 'PUT',
    });
  }

  listConsents(options: AccountConsentPageOptions = {}): Promise<AccountConsentHistory> {
    const params = new URLSearchParams();
    if (options.limit !== undefined) params.set('limit', String(options.limit));
    if (options.cursor) params.set('cursor', options.cursor);
    const query = params.toString();
    return this.transport.request(
      `/api/v1/consents${query ? `?${query}` : ''}`,
      AccountConsentHistorySchema,
      { method: 'GET', signal: options.signal },
    );
  }

  grantConsent(
    consent: AccountConsentInput,
    options?: AccountRequestOptions,
  ): Promise<AccountConsent> {
    return this.transport.request('/api/v1/consents', AccountConsentSchema, {
      ...options,
      body: consent,
      method: 'POST',
    });
  }

  revokeConsent(consent: AccountConsentInput, options?: AccountRequestOptions): Promise<void> {
    return this.transport
      .request('/api/v1/consents', EmptyResponseSchema, {
        ...options,
        body: consent,
        method: 'DELETE',
      })
      .then(() => undefined);
  }

  getGpcStatus(options?: AccountRequestOptions): Promise<AccountGpcStatus> {
    return this.transport.request('/api/v1/privacy/gpc', AccountGpcStatusSchema, {
      ...options,
      method: 'GET',
    });
  }

  listSessions(options: AccountSessionPageOptions = {}): Promise<AccountSessionsPage> {
    const params = new URLSearchParams();
    if (options.limit !== undefined) params.set('limit', String(options.limit));
    if (options.cursor) params.set('cursor', options.cursor);
    const query = params.toString();
    return this.transport.request(
      `/api/v1/security/sessions${query ? `?${query}` : ''}`,
      AccountSessionsPageSchema,
      { method: 'GET', signal: options.signal },
    );
  }

  revokeSession(sessionId: string, options?: AccountRequestOptions): Promise<void> {
    return this.success(
      `/api/v1/security/sessions/${encodeURIComponent(sessionId)}`,
      'DELETE',
      options,
    );
  }

  requestDataExport(options?: AccountRequestOptions): Promise<AccountExportRequest> {
    return this.transport.request('/api/v1/privacy/exports', AccountExportRequestSchema, {
      ...options,
      method: 'POST',
    });
  }

  getLatestDataExport(options?: AccountRequestOptions): Promise<AccountExportStatus> {
    return this.transport.request('/api/v1/privacy/exports/latest', AccountExportStatusSchema, {
      ...options,
      method: 'GET',
    });
  }

  downloadDataExport(exportId: string, options?: AccountRequestOptions): Promise<Blob> {
    return this.transport.requestBlob(
      `/api/v1/privacy/exports/${encodeURIComponent(exportId)}/document`,
      options,
    );
  }

  closeAccount(options?: AccountRequestOptions): Promise<AccountClosure> {
    return this.transport.request('/api/v1/closure', AccountClosureSchema, {
      ...options,
      method: 'POST',
    });
  }

  getAccountClosure(options?: AccountRequestOptions): Promise<AccountClosureStatus> {
    return this.transport.request('/api/v1/closure', AccountClosureStatusSchema, {
      ...options,
      method: 'GET',
    });
  }

  private success(
    path: string,
    method: 'DELETE' | 'POST',
    options?: AccountRequestOptions,
  ): Promise<void> {
    return this.transport
      .request(path, AccountSuccessSchema, { ...options, method })
      .then(() => undefined);
  }
}

export function createAccountClient(options: AccountClientOptions): AccountClient {
  return new AccountClient(options);
}
