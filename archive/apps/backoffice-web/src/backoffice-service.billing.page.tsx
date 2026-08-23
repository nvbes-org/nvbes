import { useMemo, useState } from 'react';
import { CredentialsPanel } from './backoffice-service.credentials';
import {
  credentialsReady,
  emptyCredentials,
  loadCredentials,
  saveCredentials,
} from './backoffice-service.credentials.state';
import { BillingActions } from './backoffice-service.billing.actions';
import { BillingExports } from './backoffice-service.billing.exports';
import { BillingOverviewCards } from './backoffice-service.billing.overview';
import { BillingPlatformCenterPanel } from './backoffice-service.billing-platform-center';
import { ProviderEventFailuresPanel } from './backoffice-service.billing.provider-events';
import { BillingRunbooks } from './backoffice-service.billing.runbooks';
import { BillingSearch } from './backoffice-service.billing.search';
import { AccessCenterPanel } from './backoffice-service.access-center';
import { AuditEvidenceCenterPanel } from './backoffice-service.audit-evidence-center';
import { AuditEventsPanel } from './backoffice-service.audit.events';
import { CommunicationsCenterPanel } from './backoffice-service.communications-center';
import { ComplianceCenterPanel } from './backoffice-service.compliance-center';
import { CommandCenterPanel } from './backoffice-service.command-center';
import { CustomerCenterPanel } from './backoffice-service.customer-center';
import { DeveloperCenterPanel } from './backoffice-service.developer-center';
import { EntitlementsCenterPanel } from './backoffice-service.entitlements-center';
import { GlobalSearchPanel } from './backoffice-service.global-search';
import { IdentityGovernanceCenterPanel } from './backoffice-service.identity-governance-center';
import { OperationsCenterPanel } from './backoffice-service.operations-center';
import { PendingApprovalsPanel } from './backoffice-service.pending-approvals';
import { RegionCenterPanel } from './backoffice-service.region-center';
import { RevenueCenterPanel } from './backoffice-service.revenue-center';
import { RiskDecisionCenterPanel } from './backoffice-service.risk-decision-center';
import { SecurityCenterPanel } from './backoffice-service.security-center';
import { BackofficeServiceShell } from './backoffice-service.shell';
import { StatusStrip } from './backoffice-service.status-strip';
import type { ProviderEventFailure } from './backoffice-service.types';
import { TenantDetailPanel } from './backoffice-service.tenant-detail';
import { UsageCenterPanel } from './backoffice-service.usage-center';
import { UserDetailPanel } from './backoffice-service.user-detail';
import { WorkspaceDetailPanel } from './backoffice-service.workspace-detail';

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
    const required = [
      credentials.workspaceId,
      credentials.internalToken,
      credentials.actorPrincipalId,
      credentials.backofficeRole,
    ];
    const filled = required.filter(Boolean).length;
    return Math.round((filled / required.length) * 100);
  }, [credentials]);

  function updateCredentials(nextCredentials: typeof credentials) {
    setCredentials(nextCredentials);
    saveCredentials(nextCredentials);
  }

  return (
    <BackofficeServiceShell>
      <StatusStrip isReady={isReady} readiness={readiness} />
      <CommandCenterPanel credentials={credentials} disabled={!isReady} />
      <PendingApprovalsPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
        onSelectWorkspace={setSelectedWorkspaceId}
      />
      <RevenueCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
      />
      <BillingPlatformCenterPanel
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
      <EntitlementsCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectWorkspace={setSelectedWorkspaceId}
      />
      <UsageCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectWorkspace={setSelectedWorkspaceId}
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
      <AuditEvidenceCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
      />
      <IdentityGovernanceCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
      />
      <RiskDecisionCenterPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
        onSelectUser={setSelectedUserId}
        onSelectWorkspace={setSelectedWorkspaceId}
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
          <AuditEventsPanel
            credentials={credentials}
            disabled={!isReady}
            onSelectTenant={setSelectedTenantId}
            onSelectUser={setSelectedUserId}
            onSelectWorkspace={setSelectedWorkspaceId}
          />
          <BillingActions credentials={credentials} disabled={!isReady} replayDraft={replayDraft} />
          <BillingRunbooks credentials={credentials} disabled={!isReady} />
        </div>
      </div>
    </BackofficeServiceShell>
  );
}

function replayDraftFromFailure(event: ProviderEventFailure): ReplayDraft {
  return {
    nonce: Date.now(),
    provider: event.provider,
    providerEventId: event.provider_event_id,
  };
}
