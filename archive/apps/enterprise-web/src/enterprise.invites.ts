import {
  EnterpriseInvitationInputSchema,
  type EnterpriseInvitationInput,
  type EnterpriseModuleGrant,
  type EnterpriseRole,
} from '@nvbes/identity-client';

export type BuildEnterpriseInvitationInputParams = {
  emailInput: string;
  role: EnterpriseRole;
  module_grants: EnterpriseModuleGrant[];
  workspace_ids: string[];
};

export function parseInvitationEmails(emailInput: string): string[] {
  const emails = emailInput
    .split(/[,\n]/)
    .map((email) => email.trim())
    .filter((email) => email.length > 0);

  if (emails.length === 0) {
    throw new Error('At least one invitation recipient is required.');
  }

  return emails;
}

export function buildEnterpriseInvitationInput(
  params: BuildEnterpriseInvitationInputParams,
): EnterpriseInvitationInput {
  return EnterpriseInvitationInputSchema.parse({
    emails: parseInvitationEmails(params.emailInput),
    role: params.role,
    module_grants: params.module_grants,
    workspace_ids: params.workspace_ids,
  });
}
