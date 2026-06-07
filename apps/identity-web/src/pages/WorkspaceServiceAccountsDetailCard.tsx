import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { ServiceAccount, ServiceAccountClient } from '../identity.service-accounts.api';
import {
  DetailHeaderActions,
  EmptyState,
  OAuthClientsSection,
  ServiceAccountConfiguration,
  ServiceAccountLifecycle,
  ServiceAccountOverview,
} from './WorkspaceServiceAccountsDetailCard.sections';

export function WorkspaceServiceAccountsDetailCard({
  selectedServiceAccount,
  updateName,
  setUpdateName,
  updateDescription,
  setUpdateDescription,
  updateRole,
  setUpdateRole,
  busyAction,
  editError,
  onSave,
  onToggleLifecycle,
  onOpenCreateClient,
  onOpenAttachClient,
  onRotateClientSecret,
  onRevokeClient,
}: {
  selectedServiceAccount: ServiceAccount | null;
  updateName: string;
  setUpdateName: (value: string) => void;
  updateDescription: string;
  setUpdateDescription: (value: string) => void;
  updateRole: string;
  setUpdateRole: (value: string) => void;
  busyAction: string | null;
  editError: string | null;
  onSave: () => void;
  onToggleLifecycle: () => void;
  onOpenCreateClient: () => void;
  onOpenAttachClient: () => void;
  onRotateClientSecret: (client: ServiceAccountClient) => void;
  onRevokeClient: (client: ServiceAccountClient) => void;
}) {
  return (
    <Card className="min-h-[28rem]">
      <CardHeader>
        <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
          <div>
            <CardTitle>Detail du principal</CardTitle>
            <CardDescription>
              {selectedServiceAccount
                ? 'Modifier le role, suspendre ou gerer les clients OAuth.'
                : 'Selectionnez un service account pour afficher ses actions.'}
            </CardDescription>
          </div>
          <DetailHeaderActions
            selectedServiceAccount={selectedServiceAccount}
            onToggleLifecycle={onToggleLifecycle}
            onOpenCreateClient={onOpenCreateClient}
          />
        </div>
      </CardHeader>

      <CardContent className="flex flex-col gap-6">
        {!selectedServiceAccount ? (
          <EmptyState />
        ) : (
          <>
            <ServiceAccountOverview selectedServiceAccount={selectedServiceAccount} />

            <div className="grid gap-4 xl:grid-cols-[1fr_320px]">
              <ServiceAccountConfiguration
                selectedServiceAccount={selectedServiceAccount}
                updateName={updateName}
                setUpdateName={setUpdateName}
                updateDescription={updateDescription}
                setUpdateDescription={setUpdateDescription}
                updateRole={updateRole}
                setUpdateRole={setUpdateRole}
                busyAction={busyAction}
                editError={editError}
                onSave={onSave}
              />

              <ServiceAccountLifecycle
                selectedServiceAccount={selectedServiceAccount}
                onOpenAttachClient={onOpenAttachClient}
              />
            </div>

            <OAuthClientsSection
              selectedServiceAccount={selectedServiceAccount}
              onOpenAttachClient={onOpenAttachClient}
              onOpenCreateClient={onOpenCreateClient}
              onRotateClientSecret={onRotateClientSecret}
              onRevokeClient={onRevokeClient}
            />
          </>
        )}
      </CardContent>
    </Card>
  );
}
