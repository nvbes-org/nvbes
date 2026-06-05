import { useVirtualizer } from '@tanstack/react-virtual';
import { KeyRound, Loader2, Plus, Shield, ShieldAlert, ShieldCheck, UserCog } from 'lucide-react';
import { useRef } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import type { ServiceAccount, ServiceAccountClient } from '../identity.service-accounts.api';
import {
  badgeVariantForStatus,
  formatDateTime,
  statusLabel,
} from './WorkspaceServiceAccounts.helpers';

function EmptyState() {
  return (
    <div className="flex flex-col items-center gap-3 rounded-2xl border border-dashed border-border/70 px-6 py-10 text-center">
      <div className="flex size-12 items-center justify-center rounded-full bg-muted">
        <UserCog className="size-5 text-muted-foreground" />
      </div>
      <div className="flex flex-col gap-1">
        <p className="text-sm font-medium">Aucun principal selectionne</p>
        <p className="text-sm text-muted-foreground">
          Choisissez un service account dans la liste pour voir ses details.
        </p>
      </div>
    </div>
  );
}

function MetaCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
      <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">{label}</p>
      <p className="mt-2 break-all text-sm font-medium">{value}</p>
    </div>
  );
}

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
  const clientsRef = useRef<HTMLDivElement>(null);
  const clientRowCount =
    selectedServiceAccount && selectedServiceAccount.oauth_clients.length > 0
      ? selectedServiceAccount.oauth_clients.length * 2 - 1
      : 0;
  const clientVirtualizer = useVirtualizer({
    count: clientRowCount,
    getScrollElement: () => clientsRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 12 : 148),
    overscan: 6,
  });

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
          {selectedServiceAccount ? (
            <div className="flex flex-wrap gap-2">
              <Button variant="outline" onClick={onToggleLifecycle}>
                {selectedServiceAccount.status === 'active' ? (
                  <>
                    <ShieldAlert className="size-4" />
                    Suspendre
                  </>
                ) : (
                  <>
                    <ShieldCheck className="size-4" />
                    Reactiver
                  </>
                )}
              </Button>
              <Button onClick={onOpenCreateClient}>
                <Plus className="size-4" />
                Nouveau client
              </Button>
            </div>
          ) : null}
        </div>
      </CardHeader>

      <CardContent className="flex flex-col gap-6">
        {!selectedServiceAccount ? (
          <EmptyState />
        ) : (
          <>
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              <MetaCard label="Principal ID" value={selectedServiceAccount.principal_id} />
              <MetaCard label="Tenant" value={selectedServiceAccount.tenant_id} />
              <MetaCard
                label="Organization"
                value={selectedServiceAccount.organization_id ?? 'Aucune'}
              />
              <MetaCard label="Workspace" value={selectedServiceAccount.workspace_id} />
            </div>

            <div className="grid gap-4 xl:grid-cols-[1fr_320px]">
              <div className="rounded-3xl border border-border/70 bg-background p-5">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-sm font-medium">Configuration</p>
                    <p className="text-sm text-muted-foreground">
                      Mettre a jour le nom, la description et le role RBAC.
                    </p>
                  </div>
                  <Badge variant={badgeVariantForStatus(selectedServiceAccount.status)}>
                    {statusLabel(selectedServiceAccount.status)}
                  </Badge>
                </div>

                <div className="mt-4 grid gap-4">
                  <div className="grid gap-2">
                    <label htmlFor="service-account-name" className="text-sm font-medium">
                      Nom
                    </label>
                    <input
                      id="service-account-name"
                      className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                      value={updateName}
                      onChange={(event) => setUpdateName(event.target.value)}
                    />
                  </div>

                  <div className="grid gap-2">
                    <label htmlFor="service-account-description" className="text-sm font-medium">
                      Description
                    </label>
                    <textarea
                      id="service-account-description"
                      className="flex min-h-24 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                      value={updateDescription}
                      onChange={(event) => setUpdateDescription(event.target.value)}
                    />
                  </div>

                  <div className="grid gap-2">
                    <label htmlFor="service-account-role" className="text-sm font-medium">
                      Role RBAC
                    </label>
                    <select
                      id="service-account-role"
                      className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                      value={updateRole}
                      onChange={(event) => setUpdateRole(event.target.value)}
                    >
                      <option value="viewer">viewer</option>
                      <option value="member">member</option>
                      <option value="admin">admin</option>
                      <option value="owner">owner</option>
                    </select>
                  </div>

                  {editError && (
                    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
                      {editError}
                    </div>
                  )}
                </div>

                <div className="mt-4 flex flex-wrap gap-2">
                  <Button onClick={onSave} disabled={busyAction === 'update-service-account'}>
                    {busyAction === 'update-service-account' ? (
                      <>
                        <Loader2 className="size-4 animate-spin" />
                        Sauvegarde...
                      </>
                    ) : (
                      <>
                        <Shield className="size-4" />
                        Sauvegarder
                      </>
                    )}
                  </Button>
                </div>
              </div>

              <div className="rounded-3xl border border-border/70 bg-background p-5">
                <p className="text-sm font-medium">Cycle de vie</p>
                <div className="mt-4 flex flex-col gap-3 text-sm">
                  <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
                    <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                      Cree le
                    </p>
                    <p className="mt-1 font-medium">
                      {formatDateTime(selectedServiceAccount.created_at)}
                    </p>
                  </div>
                  <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
                    <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                      Mis a jour
                    </p>
                    <p className="mt-1 font-medium">
                      {formatDateTime(selectedServiceAccount.updated_at)}
                    </p>
                  </div>
                  <div className="rounded-2xl border border-border/70 bg-muted/20 p-4">
                    <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
                      Statut
                    </p>
                    <p className="mt-1 font-medium">{statusLabel(selectedServiceAccount.status)}</p>
                  </div>
                  <Button variant="outline" onClick={onOpenAttachClient}>
                    <KeyRound className="size-4" />
                    Attacher un client existant
                  </Button>
                </div>
              </div>
            </div>

            <div>
              <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
                <div>
                  <h3 className="text-base font-semibold">Clients OAuth</h3>
                  <p className="text-sm text-muted-foreground">
                    Secrets visibles une seule fois, rotation/revocation immediates.
                  </p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button variant="outline" onClick={onOpenAttachClient}>
                    <KeyRound className="size-4" />
                    Attacher un client
                  </Button>
                  <Button onClick={onOpenCreateClient}>
                    <Plus className="size-4" />
                    Creer un client
                  </Button>
                </div>
              </div>

              <div ref={clientsRef} className="mt-4 max-h-[28rem] overflow-auto p-0">
                {selectedServiceAccount.oauth_clients.length === 0 ? (
                  <div className="flex flex-col items-center gap-3 rounded-2xl border border-dashed border-border/70 px-6 py-10 text-center">
                    <div className="flex size-12 items-center justify-center rounded-full bg-muted">
                      <KeyRound className="size-5 text-muted-foreground" />
                    </div>
                    <div className="flex flex-col gap-1">
                      <p className="text-sm font-medium">Aucun client OAuth</p>
                      <p className="text-sm text-muted-foreground">
                        Creer ou attacher un client pour emettre des tokens M2M.
                      </p>
                    </div>
                  </div>
                ) : (
                  <div
                    style={{
                      height: `${clientVirtualizer.getTotalSize()}px`,
                      position: 'relative',
                    }}
                  >
                    {clientVirtualizer.getVirtualItems().map((virtualItem) => {
                      if (virtualItem.index % 2 === 1) {
                        return (
                          <div
                            key={`separator-${virtualItem.index}`}
                            className="absolute left-0 right-0"
                            style={{ transform: `translateY(${virtualItem.start}px)` }}
                          >
                            <Separator className="my-2" />
                          </div>
                        );
                      }

                      const client =
                        selectedServiceAccount.oauth_clients[Math.floor(virtualItem.index / 2)];

                      return (
                        <div
                          key={client.id}
                          ref={clientVirtualizer.measureElement}
                          data-index={virtualItem.index}
                          className="absolute left-0 right-0"
                          style={{ transform: `translateY(${virtualItem.start}px)` }}
                        >
                          <div className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-muted/10 p-4 xl:flex-row xl:items-start xl:justify-between">
                            <div className="min-w-0 flex-1">
                              <div className="flex flex-wrap items-center gap-2">
                                <p className="truncate text-sm font-medium">{client.name}</p>
                                <Badge variant={client.revoked_at ? 'destructive' : 'secondary'}>
                                  {client.revoked_at ? 'Revoque' : 'Actif'}
                                </Badge>
                                <Badge variant="outline">{client.client_id}</Badge>
                              </div>
                              <div className="mt-2 flex flex-wrap gap-2">
                                <Badge variant="secondary">
                                  {client.client_assertion_required
                                    ? 'client assertion requise'
                                    : 'assertion optionnelle'}
                                </Badge>
                                <Badge variant="outline">
                                  {client.client_assertion_public_key_configured
                                    ? 'JWK configure'
                                    : 'pas de JWK'}
                                </Badge>
                                <Badge variant="outline">{client.required_acr}</Badge>
                              </div>
                              <div className="mt-3 flex flex-wrap gap-1.5">
                                {client.allowed_scopes.map((scope) => (
                                  <Badge key={scope} variant="ghost" className="h-6 rounded-full">
                                    {scope}
                                  </Badge>
                                ))}
                              </div>
                            </div>
                            <div className="flex flex-wrap gap-2">
                              <Button
                                variant="outline"
                                onClick={() => onRotateClientSecret(client)}
                              >
                                Rotation
                              </Button>
                              <Button variant="destructive" onClick={() => onRevokeClient(client)}>
                                Revoquer
                              </Button>
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
