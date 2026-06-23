import { useMemo, useState } from 'react';
import { CredentialsPanel } from './internal-admin.credentials';
import {
  credentialsReady,
  emptyCredentials,
  loadCredentials,
  saveCredentials,
} from './internal-admin.credentials.state';
import { BillingActions } from './internal-admin.billing.actions';
import { BillingExports } from './internal-admin.billing.exports';
import { BillingOverviewCards } from './internal-admin.billing.overview';
import { ProviderEventFailuresPanel } from './internal-admin.billing.provider-events';
import { BillingRunbooks } from './internal-admin.billing.runbooks';
import { BillingSearch } from './internal-admin.billing.search';
import { AccessCenterPanel } from './internal-admin.access-center';
import { AuditEventsPanel } from './internal-admin.audit.events';
import { CommunicationsCenterPanel } from './internal-admin.communications-center';
import { ComplianceCenterPanel } from './internal-admin.compliance-center';
import { CommandCenterPanel } from './internal-admin.command-center';
import { CustomerCenterPanel } from './internal-admin.customer-center';
import { DeveloperCenterPanel } from './internal-admin.developer-center';
import { GlobalSearchPanel } from './internal-admin.global-search';
import { IdentityGovernanceCenterPanel } from './internal-admin.identity-governance-center';
import { OperationsCenterPanel } from './internal-admin.operations-center';
import { RegionCenterPanel } from './internal-admin.region-center';
import { RevenueCenterPanel } from './internal-admin.revenue-center';
import { SecurityCenterPanel } from './internal-admin.security-center';
import { InternalAdminShell } from './internal-admin.shell';
import { StatusStrip } from './internal-admin.status-strip';
import type { ProviderEventFailure } from './internal-admin.types';
import { TenantDetailPanel } from './internal-admin.tenant-detail';
import { UserDetailPanel } from './internal-admin.user-detail';
import { WorkspaceDetailPanel } from './internal-admin.workspace-detail';

type ReplayDraft = {
  nonce: number;
  provider: string;
  providerEventId: string;
};

export function BillingOperationsPage() {
  const [credentials, setCredentials] = useState(loadCredentials);
  const [replayDraft, setReplayDraft] = useState<ReplayDraft | null>(null);
  const [selectedTenantId, setSelectedTenantId] = useState<string | null>(null);
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(null);
  const isReady = credentialsReady(credentials);
  const readiness = useMemo(() => {
    const filled = Object.values(credentials).filter(Boolean).length;
    return Math.round((filled / 3) * 100);
  }, [credentials]);

  function updateCredentials(nextCredentials: typeof credentials) {
    setCredentials(nextCredentials);
    saveCredentials(nextCredentials);
  }

  return (
    <InternalAdminShell>
      <StatusStrip isReady={isReady} readiness={readiness} />
      <CommandCenterPanel credentials={credentials} disabled={!isReady} />
      <RevenueCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
      />
      <CustomerCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectWorkspace={setSelectedWorkspaceId}
      />
      <DeveloperCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
      />
      <OperationsCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
      />
      <AccessCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
        onSelectWorkspace={setSelectedWorkspaceId}
      />
      <IdentityGovernanceCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
      />
      <SecurityCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
      />
      <ComplianceCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
      />
      <CommunicationsCenterPanel credentials={credentials} disabled={!isReady} />
      <RegionCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectWorkspace={setSelectedWorkspaceId}
      />
      <GlobalSearchPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
        onSelectWorkspace={setSelectedWorkspaceId}
      />
      <TenantDetailPanel
        credentials={credentials}
        disabled={!isReady}
        tenantId={selectedTenantId}
      />
      <WorkspaceDetailPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        workspaceId={selectedWorkspaceId}
      />
      <UserDetailPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectWorkspace={setSelectedWorkspaceId}
        principalId={selectedUserId}
      />
      <div className="grid gap-5 xl:grid-cols-[360px_1fr]">
        <div className="space-y-5">
          <CredentialsPanel
            credentials={credentials}
            isReady={isReady}
            onChange={updateCredentials}
            onReset={() => updateCredentials(emptyCredentials)}
          />
          <BillingOverviewCards
            credentials={credentials}
            disabled={!isReady}
            readiness={readiness}
          />
          <BillingExports credentials={credentials} disabled={!isReady} />
        </div>
        <div className="space-y-5">
          <BillingSearch credentials={credentials} disabled={!isReady} />
          <ProviderEventFailuresPanel
            credentials={credentials}
            disabled={!isReady}
            onReplay={(event) => {
              setReplayDraft(replayDraftFromFailure(event));
              window.location.hash = 'mutations';
            }}
          />
          <AuditEventsPanel credentials={credentials} disabled={!isReady} />
          <BillingActions credentials={credentials} disabled={!isReady} replayDraft={replayDraft} />
          <BillingRunbooks credentials={credentials} disabled={!isReady} />
        </div>
      </div>
    </InternalAdminShell>
  );
}

function replayDraftFromFailure(event: ProviderEventFailure): ReplayDraft {
  return {
    nonce: Date.now(),
    provider: event.provider,
    providerEventId: event.provider_event_id,
  };
}
