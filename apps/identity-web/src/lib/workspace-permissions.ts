export function canManageServiceAccounts(workspaceRole: string | null | undefined): boolean {
  return workspaceRole === 'owner' || workspaceRole === 'admin';
}
