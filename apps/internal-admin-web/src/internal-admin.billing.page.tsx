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
import { AuditEventsPanel } from './internal-admin.audit.events';
import { CommandCenterPanel } from './internal-admin.command-center';
import { GlobalSearchPanel } from './internal-admin.global-search';
import { InternalAdminShell } from './internal-admin.shell';
import { StatusStrip } from './internal-admin.status-strip';
import type { ProviderEventFailure } from './internal-admin.types';
import { TenantDetailPanel } from './internal-admin.tenant-detail';

type ReplayDraft = {
  nonce: number;
  provider: string;
  providerEventId: string;
};

export function BillingOperationsPage() {
  const [credentials, setCredentials] = useState(loadCredentials);
  const [replayDraft, setReplayDraft] = useState<ReplayDraft | null>(null);
  const [selectedTenantId, setSelectedTenantId] = useState<string | null>(null);
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
      <GlobalSearchPanel
        credentials={credentials}
        disabled={!isReady}
        onSelectTenant={setSelectedTenantId}
      />
      <TenantDetailPanel
        credentials={credentials}
        disabled={!isReady}
        tenantId={selectedTenantId}
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
