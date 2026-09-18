import { describe, expect, it } from 'vite-plus/test';
import {
  AccountClosureParticipantSchema,
  AccountClosureSchema,
  AccountClosureStatusSchema,
  AccountCreateTeamInputSchema,
  AccountCreatedTeamSchema,
  AccountExportParticipantSchema,
  AccountExportRequestSchema,
  AccountExportStatusSchema,
  AccountJoinTeamInputSchema,
  AccountLanguageSchema,
  AccountNotificationsSchema,
  AccountPreferencesSchema,
  AccountProfileSchema,
  AccountTeamSchema,
  AccountTeamsEnvelopeSchema,
  AccountThemeSchema,
  AccountUpdateProfileInputSchema,
} from './account.schemas';

describe('Account schema contracts', () => {
  it('keeps every public object field', () => {
    expect(Object.keys(AccountProfileSchema.shape)).toEqual([
      'id',
      'display_name',
      'firstname',
      'lastname',
      'username',
      'birthdate',
      'region',
      'created_at',
    ]);
    expect(Object.keys(AccountUpdateProfileInputSchema.shape)).toEqual([
      'firstname',
      'lastname',
      'username',
      'birthdate',
      'region',
    ]);
    expect(Object.keys(AccountPreferencesSchema.shape)).toEqual(['theme', 'language']);
    expect(Object.keys(AccountNotificationsSchema.shape)).toEqual([
      'email',
      'push',
      'in_app',
      'marketing_email',
    ]);
    expect(Object.keys(AccountTeamSchema.shape)).toEqual(['id', 'name', 'role', 'created_at']);
    expect(Object.keys(AccountTeamsEnvelopeSchema.shape)).toEqual(['teams']);
    expect(Object.keys(AccountCreateTeamInputSchema.shape)).toEqual(['name']);
    expect(Object.keys(AccountCreatedTeamSchema.shape)).toEqual([
      'id',
      'name',
      'role',
      'created_at',
      'join_code',
    ]);
    expect(Object.keys(AccountJoinTeamInputSchema.shape)).toEqual(['join_code']);
    expect(Object.keys(AccountExportRequestSchema.shape)).toEqual([
      'export_id',
      'status',
      'requested_at',
    ]);
    expect(Object.keys(AccountExportParticipantSchema.shape)).toEqual([
      'participant',
      'status',
      'attempts',
      'completed_at',
      'last_error',
    ]);
    expect(Object.keys(AccountExportStatusSchema.shape)).toEqual([
      'export_id',
      'status',
      'requested_at',
      'updated_at',
      'completed_at',
      'expires_at',
      'last_error',
      'participants',
    ]);
    expect(Object.keys(AccountClosureSchema.shape)).toEqual(['saga_id', 'status', 'requested_at']);
    expect(Object.keys(AccountClosureParticipantSchema.shape)).toEqual([
      'participant',
      'status',
      'attempts',
      'completed_at',
      'last_error',
    ]);
    expect(Object.keys(AccountClosureStatusSchema.shape)).toEqual([
      'saga_id',
      'status',
      'requested_at',
      'updated_at',
      'completed_at',
      'last_error',
      'participants',
    ]);
  });

  it('keeps every enum value used by preferences, teams and sagas', () => {
    expect(AccountThemeSchema.options).toEqual(['system', 'light', 'dark']);
    expect(AccountLanguageSchema.options).toEqual(['fr', 'en']);
    expect(AccountTeamSchema.shape.role.options).toEqual(['owner', 'member']);
    expect(AccountExportRequestSchema.shape.status.options).toEqual(['pending', 'processing']);
    expect(AccountExportParticipantSchema.shape.participant.options).toEqual([
      'identity',
      'account',
      'billing',
      'email',
      'trust_risk',
      'platform_operations',
    ]);
    expect(AccountExportParticipantSchema.shape.status.options).toEqual([
      'pending',
      'processing',
      'completed',
      'failed',
      'cancelled',
      'expired',
    ]);
    expect(AccountExportStatusSchema.shape.status.options).toEqual([
      'pending',
      'processing',
      'completed',
      'failed',
      'expired',
    ]);
    expect(AccountClosureSchema.shape.status.options).toEqual(['pending', 'processing']);
    expect(AccountClosureStatusSchema.shape.status.options).toEqual([
      'pending',
      'processing',
      'completed',
      'failed',
      'cancelled',
    ]);
  });

  it('enforces profile update length boundaries', () => {
    const valid = {
      birthdate: null,
      firstname: 'A',
      lastname: 'B',
      region: 'FRA',
      username: 'user',
    };
    expect(AccountUpdateProfileInputSchema.parse(valid)).toEqual(valid);
    for (const [field, value] of [
      ['firstname', 'a'.repeat(101)],
      ['lastname', 'a'.repeat(101)],
      ['username', 'ab'],
      ['username', 'a'.repeat(101)],
      ['region', 'F'],
      ['region', 'a'.repeat(33)],
    ] as const) {
      expect(AccountUpdateProfileInputSchema.safeParse({ ...valid, [field]: value }).success).toBe(
        false,
      );
    }
  });

  it('enforces team name length boundaries', () => {
    expect(AccountCreateTeamInputSchema.parse({ name: 'Team' })).toEqual({ name: 'Team' });
    expect(AccountCreateTeamInputSchema.safeParse({ name: '' }).success).toBe(false);
    expect(AccountCreateTeamInputSchema.safeParse({ name: 'a'.repeat(101) }).success).toBe(false);
  });
});
