import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  Calendar,
  Check,
  CheckCircle2,
  Clock,
  Copy,
  History,
  KeyRound,
  Loader2,
  RotateCcwKey,
  ShieldAlert,
  Trash2,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import {
  listDeveloperOAuthClients,
  listDeveloperSecretVersions,
  rotateDeveloperSecret,
  revokeDeveloperSecretVersion,
} from '../developer.api';
import type { RotateDeveloperSecret } from '../developer.schemas';

export function SecretsPage() {
  const queryClient = useQueryClient();

  // State variables
  const [selectedClientId, setSelectedClientId] = useState('');
  const [overlapHours, setOverlapHours] = useState(24);
  const [understandChecked, setUnderstandChecked] = useState(false);
  const [newSecretResult, setNewSecretResult] = useState<RotateDeveloperSecret | null>(null);
  const [copied, setCopied] = useState(false);

  // Queries
  const clientsQuery = useQuery({
    queryKey: ['developer-oauth-clients'],
    queryFn: ({ signal }) => listDeveloperOAuthClients(signal),
    staleTime: 30_000,
  });

  const clientList = clientsQuery.data || [];
  const activeClientId = selectedClientId || clientList[0]?.client_id || '';
  const activeClientName = clientList.find((c) => c.client_id === activeClientId)?.name || '';

  const versionsQuery = useQuery({
    queryKey: ['developer-client-secret-versions', activeClientId],
    queryFn: ({ signal }) => listDeveloperSecretVersions(activeClientId, signal),
    enabled: activeClientId.length > 0,
    staleTime: 5_000,
  });

  // Reset states on client change
  useEffect(() => {
    setNewSecretResult(null);
    setUnderstandChecked(false);
    setCopied(false);
  }, [activeClientId]);

  // Mutations
  const rotateMutation = useMutation({
    mutationFn: () => rotateDeveloperSecret(activeClientId, { overlapHours }),
    onSuccess: (data) => {
      setNewSecretResult(data);
      setUnderstandChecked(false);
      void queryClient.invalidateQueries({
        queryKey: ['developer-client-secret-versions', activeClientId],
      });
    },
  });

  const revokeMutation = useMutation({
    mutationFn: (versionId: string) => revokeDeveloperSecretVersion(activeClientId, versionId),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ['developer-client-secret-versions', activeClientId],
      });
    },
  });

  // Copy helper
  const handleCopy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy secret', err);
    }
  };

  if (clientsQuery.isLoading) {
    return <div className="h-80 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (clientsQuery.isError || clientList.length === 0) {
    return (
      <div className="rounded-lg border border-border bg-card p-6">
        <div className="flex items-center gap-3 text-red-600">
          <AlertTriangle className="h-5 w-5" />
          <h2 className="text-base font-semibold">No clients registered</h2>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          Please register an OAuth client in the dashboard first to configure secret rotation.
        </p>
      </div>
    );
  }

  const versions = versionsQuery.data || [];

  return (
    <section className="space-y-6">
      {/* Page Header */}
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <RotateCcwKey className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold text-foreground">Secret rotation</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Configure, preview, and perform zero-downtime client secret rotations with overlap
            periods.
          </p>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-[400px_1fr]">
        {/* Left column: Configuration Panel */}
        <div className="space-y-6">
          <div className="rounded-lg border border-border bg-card p-5 space-y-4">
            <h3 className="font-semibold text-foreground">Guided configuration</h3>

            {/* Target Client dropdown */}
            <label className="grid gap-2 text-sm font-medium">
              Target OAuth client
              <select
                className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                value={activeClientId}
                onChange={(event) => setSelectedClientId(event.currentTarget.value)}
              >
                {clientList.map((client) => (
                  <option key={client.client_id} value={client.client_id}>
                    {client.name} ({client.client_id.slice(0, 12)}...)
                  </option>
                ))}
              </select>
            </label>

            {/* Overlap Window input */}
            <label className="grid gap-2 text-sm font-medium">
              Overlap window (hours)
              <div className="relative">
                <input
                  className="h-10 w-full rounded-md border border-input bg-background pl-3 pr-12 text-sm"
                  min={1}
                  max={720}
                  type="number"
                  value={overlapHours}
                  onChange={(event) => setOverlapHours(Number(event.currentTarget.value))}
                />
                <span className="absolute right-3 top-2 text-xs text-muted-foreground">hours</span>
              </div>
              <span className="text-xs text-muted-foreground">
                Both the old and new client secrets will be valid for this period.
              </span>
            </label>

            {/* Visual Timeline preview */}
            <div className="rounded-lg bg-muted/50 p-3.5 space-y-3">
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Rotation Timeline Preview
              </span>
              <div className="space-y-4 relative pl-3 border-l border-border">
                <div className="relative">
                  <div className="absolute -left-[17px] top-1 h-2 w-2 rounded-full bg-primary" />
                  <p className="text-xs font-medium text-foreground">Now: Rotate Secret</p>
                  <p className="text-[11px] text-muted-foreground">
                    New secret is generated and active immediately.
                  </p>
                </div>
                <div className="relative">
                  <div className="absolute -left-[17px] top-1 h-2 w-2 rounded-full bg-amber-500" />
                  <p className="text-xs font-medium text-foreground">
                    Overlap Period ({overlapHours}h)
                  </p>
                  <p className="text-[11px] text-muted-foreground">
                    Both secrets authenticate requests. Update your services.
                  </p>
                </div>
                <div className="relative">
                  <div className="absolute -left-[17px] top-1 h-2 w-2 rounded-full bg-destructive" />
                  <p className="text-xs font-medium text-foreground">In {overlapHours} hours</p>
                  <p className="text-[11px] text-muted-foreground">
                    Old secret expires automatically.
                  </p>
                </div>
              </div>
            </div>

            {/* Warning Checkbox & Submit */}
            <div className="space-y-4 pt-2">
              <label className="flex items-start gap-2 text-xs cursor-pointer select-none">
                <input
                  type="checkbox"
                  className="mt-0.5 rounded border-input text-primary focus:ring-primary h-4 w-4"
                  checked={understandChecked}
                  onChange={(event) => setUnderstandChecked(event.currentTarget.checked)}
                />
                <span className="text-muted-foreground">
                  I understand that rotating will demote the current client secret to the overlap
                  status and automatically invalidate it after {overlapHours} hours.
                </span>
              </label>

              <button
                type="button"
                onClick={() => rotateMutation.mutate()}
                disabled={!understandChecked || rotateMutation.isPending}
                className="w-full flex items-center justify-center gap-2 rounded-md bg-primary py-2 px-4 text-sm font-medium text-primary-foreground hover:bg-primary/95 disabled:opacity-50 transition-all shadow-sm"
              >
                {rotateMutation.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Rotating secret...
                  </>
                ) : (
                  <>
                    <RotateCcwKey className="h-4 w-4" />
                    Rotate Secret
                  </>
                )}
              </button>
            </div>
          </div>
        </div>

        {/* Right column: Status details & History */}
        <div className="space-y-6">
          {/* Display New Secret Success Card */}
          {newSecretResult ? (
            <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/10 p-5 space-y-4">
              <div className="flex items-start gap-3 text-emerald-600">
                <CheckCircle2 className="h-5 w-5 mt-0.5" />
                <div>
                  <h4 className="font-semibold text-emerald-950">New Client Secret Generated</h4>
                  <p className="text-sm text-emerald-800/80">
                    Your rotation is successfully scheduled. Copy the new secret now.
                  </p>
                </div>
              </div>

              {/* Monospace display */}
              <div className="flex items-center gap-2 rounded-md bg-emerald-950/5 p-3.5 font-mono text-sm break-all select-all border border-emerald-950/10">
                <span className="flex-1 text-emerald-950 font-bold select-all">
                  {newSecretResult.client_secret}
                </span>
                <button
                  type="button"
                  onClick={() => handleCopy(newSecretResult.client_secret)}
                  className="flex h-8 w-8 items-center justify-center rounded-md bg-background text-foreground hover:bg-muted border border-border shadow-sm transition-all"
                >
                  {copied ? (
                    <Check className="h-4 w-4 text-emerald-600" />
                  ) : (
                    <Copy className="h-4 w-4 text-muted-foreground" />
                  )}
                </button>
              </div>

              {/* Warnings and deadlines */}
              <div className="flex gap-2 text-xs text-emerald-800">
                <ShieldAlert className="h-4 w-4 flex-shrink-0" />
                <div className="space-y-1">
                  <p className="font-medium">Important Security Warnings:</p>
                  <ul className="list-disc pl-4 space-y-1">
                    <li>This secret will never be shown again. Save it immediately.</li>
                    <li>
                      The previous secret remains valid until{' '}
                      <span className="font-semibold">
                        {new Date(newSecretResult.overlap_ends_at).toLocaleString()}
                      </span>{' '}
                      ({overlapHours} hours from now).
                    </li>
                  </ul>
                </div>
              </div>
            </div>
          ) : null}

          {/* Secret Versions History */}
          <div className="rounded-lg border border-border bg-card p-5 space-y-4">
            <div className="flex items-center justify-between border-b border-border pb-3">
              <div className="flex items-center gap-2">
                <History className="h-4 w-4 text-muted-foreground" />
                <h3 className="font-semibold text-foreground">Secret versions history</h3>
              </div>
              <span className="text-xs text-muted-foreground font-mono">{activeClientName}</span>
            </div>

            {versionsQuery.isLoading ? (
              <div className="space-y-3">
                <div className="h-10 animate-pulse rounded bg-muted" />
                <div className="h-10 animate-pulse rounded bg-muted" />
              </div>
            ) : versionsQuery.isError ? (
              <div className="text-sm text-red-600">Failed to load secret versions.</div>
            ) : versions.length === 0 ? (
              <p className="text-sm text-muted-foreground py-4 text-center">
                No secret versions found. Standard secret is in use.
              </p>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-left text-xs">
                  <thead>
                    <tr className="border-b border-border text-muted-foreground uppercase text-[10px] tracking-wider">
                      <th className="py-2.5 font-semibold">Secret last 4</th>
                      <th className="py-2.5 font-semibold">Status</th>
                      <th className="py-2.5 font-semibold">Created / Expires</th>
                      <th className="py-2.5 font-semibold text-right">Actions</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {versions.map((v) => {
                      const isActive = v.status === 'active';
                      const isOverlap = v.status === 'overlap';

                      let badgeClass = 'bg-muted text-muted-foreground border-border';
                      if (isActive) {
                        badgeClass = 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20';
                      } else if (isOverlap) {
                        badgeClass = 'bg-amber-500/10 text-amber-500 border-amber-500/20';
                      } else if (v.status === 'revoked') {
                        badgeClass = 'bg-rose-500/10 text-rose-500 border-rose-500/20';
                      }

                      return (
                        <tr key={v.id} className="hover:bg-muted/10">
                          <td className="py-3.5">
                            <div className="flex items-center gap-1.5 font-mono text-[11px] font-medium text-foreground">
                              <KeyRound className="h-3 w-3 text-muted-foreground" />
                              •••• {v.secret_last4}
                            </div>
                          </td>
                          <td className="py-3.5">
                            <span
                              className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase ${badgeClass}`}
                            >
                              {v.status}
                            </span>
                          </td>
                          <td className="py-3.5">
                            <div className="space-y-1">
                              <div className="flex items-center gap-1 text-[11px] text-muted-foreground">
                                <Calendar className="h-3 w-3" />
                                {new Date(v.created_at).toLocaleDateString()}
                              </div>
                              {v.expires_at ? (
                                <div className="flex items-center gap-1 text-[10px] text-amber-600 font-medium">
                                  <Clock className="h-3 w-3" />
                                  Exp: {new Date(v.expires_at).toLocaleString()}
                                </div>
                              ) : null}
                              {v.revoked_at ? (
                                <div className="text-[10px] text-rose-600 font-medium">
                                  Revoked: {new Date(v.revoked_at).toLocaleDateString()}
                                </div>
                              ) : null}
                            </div>
                          </td>
                          <td className="py-3.5 text-right">
                            {(isActive || isOverlap) && !v.revoked_at ? (
                              <button
                                type="button"
                                onClick={() => revokeMutation.mutate(v.id)}
                                disabled={revokeMutation.isPending}
                                title="Revoke this secret immediately"
                                className="inline-flex items-center gap-1 rounded border border-rose-500/20 bg-rose-500/5 px-2 py-1 text-[10px] font-semibold text-rose-600 hover:bg-rose-500/10 disabled:opacity-50 transition-colors"
                              >
                                {revokeMutation.isPending && revokeMutation.variables === v.id ? (
                                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                                ) : (
                                  <Trash2 className="h-3.5 w-3.5" />
                                )}
                                Revoke
                              </button>
                            ) : (
                              <span className="text-[10px] text-muted-foreground font-medium">
                                -
                              </span>
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
