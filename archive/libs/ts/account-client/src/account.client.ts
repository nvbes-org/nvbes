import {
  type AccountClosure,
  AccountClosureSchema,
  type AccountClosureStatus,
  AccountClosureStatusSchema,
  type AccountCreateTeamInput,
  type AccountCreatedTeam,
  AccountCreatedTeamSchema,
  type AccountExportRequest,
  AccountExportRequestSchema,
  type AccountExportStatus,
  AccountExportStatusSchema,
  type AccountJoinTeamInput,
  type AccountNotifications,
  AccountNotificationsSchema,
  type AccountPreferences,
  AccountPreferencesSchema,
  type AccountProfile,
  AccountProfileEnvelopeSchema,
  type AccountTeam,
  AccountTeamSchema,
  AccountTeamsEnvelopeSchema,
  type AccountUpdateProfileInput,
  EmptyResponseSchema,
} from './account.schemas';
import {
  type AccountRequestOptions,
  AccountTransport,
  type AccountTransportOptions,
} from './account.transport';

export type AccountClientOptions = AccountTransportOptions;

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

  listTeams(options?: AccountRequestOptions): Promise<AccountTeam[]> {
    return this.transport
      .request('/api/v1/teams', AccountTeamsEnvelopeSchema, { ...options, method: 'GET' })
      .then(({ teams }) => teams);
  }

  createTeam(
    input: AccountCreateTeamInput,
    options?: AccountRequestOptions,
  ): Promise<AccountCreatedTeam> {
    return this.transport.request('/api/v1/teams', AccountCreatedTeamSchema, {
      ...options,
      body: input,
      method: 'POST',
    });
  }

  joinTeam(input: AccountJoinTeamInput, options?: AccountRequestOptions): Promise<AccountTeam> {
    return this.transport.request('/api/v1/teams/join', AccountTeamSchema, {
      ...options,
      body: input,
      method: 'POST',
    });
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

  cancelAccountClosure(options?: AccountRequestOptions): Promise<void> {
    return this.transport
      .request('/api/v1/closure/cancel', EmptyResponseSchema, { ...options, method: 'POST' })
      .then(() => undefined);
  }
}

export function createAccountClient(options: AccountClientOptions): AccountClient {
  return new AccountClient(options);
}
