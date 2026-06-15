import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Library, Plus, Search, Trash2, Edit2, X, Users, Globe } from 'lucide-react';
import { useState, useMemo } from 'react';
import {
  listDeveloperScopes,
  createDeveloperScope,
  updateDeveloperScope,
  deleteDeveloperScope,
  getDeveloperContext,
} from '../developer.api';
import { canUseDeveloperPermission } from '../developer.permissions';
import { inputClass, textareaClass, buttonClass, Field } from './Portal.shared';

type ScopeRisk = 'low' | 'medium' | 'high' | 'restricted';
type ScopeLifecycle = 'proposed' | 'active' | 'deprecated' | 'retired';

interface ScopeFormData {
  scope_key: string;
  display_name: string;
  description: string;
  risk: ScopeRisk;
  owner_team: string;
  lifecycle: ScopeLifecycle;
  allowed_audiences: string;
}

export function ScopesPage() {
  const queryClient = useQueryClient();
  const [searchTerm, setSearchTerm] = useState('');
  const [riskFilter, setRiskFilter] = useState<string>('all');
  const [lifecycleFilter, setLifecycleFilter] = useState<string>('all');

  // Modal states
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [editingScope, setEditingScope] = useState<ScopeFormData | null>(null);
  const [editingScopeKey, setEditingScopeKey] = useState<string | null>(null);
  const [deletingScopeKey, setDeletingScopeKey] = useState<string | null>(null);

  // Form error states
  const [formError, setFormError] = useState<string | null>(null);

  // Context & permissions
  const contextQuery = useQuery({
    queryKey: ['developer-context'],
    queryFn: ({ signal }) => getDeveloperContext(signal),
    staleTime: 60_000,
  });

  const canManage = useMemo(() => {
    const context = contextQuery.data;
    return context ? canUseDeveloperPermission(context, 'scopes.manage') : false;
  }, [contextQuery.data]);

  // Scopes list query
  const scopesQuery = useQuery({
    queryKey: ['developer-scopes'],
    queryFn: ({ signal }) => listDeveloperScopes(signal),
    staleTime: 60_000,
  });

  // Mutations
  const createMutation = useMutation({
    mutationFn: createDeveloperScope,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['developer-scopes'] });
      setIsCreateOpen(false);
      setFormError(null);
    },
    onError: (error: any) => {
      setFormError(error?.body?.error?.message || error?.message || 'Failed to create scope');
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ scopeKey, input }: { scopeKey: string; input: any }) =>
      updateDeveloperScope(scopeKey, input),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['developer-scopes'] });
      setEditingScope(null);
      setEditingScopeKey(null);
      setFormError(null);
    },
    onError: (error: any) => {
      setFormError(error?.body?.error?.message || error?.message || 'Failed to update scope');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: deleteDeveloperScope,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['developer-scopes'] });
      setDeletingScopeKey(null);
    },
  });

  // Filtered scopes list
  const filteredScopes = useMemo(() => {
    if (!scopesQuery.data) return [];
    return scopesQuery.data.filter((scope) => {
      const matchesSearch =
        scope.scope_key.toLowerCase().includes(searchTerm.toLowerCase()) ||
        scope.display_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        scope.description.toLowerCase().includes(searchTerm.toLowerCase());

      const matchesRisk = riskFilter === 'all' || scope.risk === riskFilter;
      const matchesLifecycle = lifecycleFilter === 'all' || scope.lifecycle === lifecycleFilter;

      return matchesSearch && matchesRisk && matchesLifecycle;
    });
  }, [scopesQuery.data, searchTerm, riskFilter, lifecycleFilter]);

  if (scopesQuery.isLoading) {
    return <div className="h-80 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (scopesQuery.isError || !scopesQuery.data) {
    return <ScopesUnavailable />;
  }

  const handleCreateSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const scope_key = (form.get('scope_key') as string).trim();
    const display_name = (form.get('display_name') as string).trim();
    const description = (form.get('description') as string).trim();
    const risk = form.get('risk') as ScopeRisk;
    const owner_team = (form.get('owner_team') as string).trim();
    const lifecycle = form.get('lifecycle') as ScopeLifecycle;
    const allowed_audiences_str = form.get('allowed_audiences') as string;

    const allowed_audiences = allowed_audiences_str
      ? allowed_audiences_str
          .split(',')
          .map((a) => a.trim())
          .filter(Boolean)
      : [];

    createMutation.mutate({
      scope_key,
      display_name,
      description,
      risk,
      owner_team,
      lifecycle,
      allowed_audiences,
    });
  };

  const handleUpdateSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!editingScopeKey) return;
    const form = new FormData(event.currentTarget);
    const display_name = (form.get('display_name') as string).trim();
    const description = (form.get('description') as string).trim();
    const risk = form.get('risk') as ScopeRisk;
    const owner_team = (form.get('owner_team') as string).trim();
    const lifecycle = form.get('lifecycle') as ScopeLifecycle;
    const allowed_audiences_str = form.get('allowed_audiences') as string;

    const allowed_audiences = allowed_audiences_str
      ? allowed_audiences_str
          .split(',')
          .map((a) => a.trim())
          .filter(Boolean)
      : [];

    updateMutation.mutate({
      scopeKey: editingScopeKey,
      input: {
        display_name,
        description,
        risk,
        owner_team,
        lifecycle,
        allowed_audiences,
      },
    });
  };

  const getRiskBadgeStyles = (risk: string) => {
    switch (risk) {
      case 'low':
        return 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20';
      case 'medium':
        return 'bg-amber-500/10 text-amber-500 border-amber-500/20';
      case 'high':
        return 'bg-orange-500/10 text-orange-500 border-orange-500/20';
      case 'restricted':
        return 'bg-rose-500/10 text-rose-500 border-rose-500/20';
      default:
        return 'bg-zinc-500/10 text-zinc-500 border-zinc-500/20';
    }
  };

  const getLifecycleBadgeStyles = (lifecycle: string) => {
    switch (lifecycle) {
      case 'proposed':
        return 'bg-sky-500/10 text-sky-500 border-sky-500/20';
      case 'active':
        return 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20';
      case 'deprecated':
        return 'bg-yellow-500/10 text-yellow-500 border-yellow-500/20';
      case 'retired':
        return 'bg-zinc-500/10 text-zinc-500 border-zinc-500/20';
      default:
        return 'bg-zinc-500/10 text-zinc-500 border-zinc-500/20';
    }
  };

  return (
    <section className="space-y-6">
      {/* Top Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-start gap-3">
          <div className="rounded-md border border-border bg-card p-2 shadow-sm">
            <Library className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h2 className="text-xl font-semibold">Scope registry</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Central catalog of OAuth scopes, risk levels, lifecycle, owners, and audiences.
            </p>
          </div>
        </div>
        {canManage && (
          <button
            type="button"
            onClick={() => {
              setFormError(null);
              setIsCreateOpen(true);
            }}
            className={`${buttonClass} gap-2 shadow-sm hover:opacity-90 transition-opacity`}
          >
            <Plus className="h-4 w-4" />
            Register scope
          </button>
        )}
      </div>

      {/* Search & Filters */}
      <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4 md:flex-row md:items-center">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search by scope key, display name, description..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className={`${inputClass} w-full pl-9`}
          />
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium text-muted-foreground">Risk:</span>
            <select
              value={riskFilter}
              onChange={(e) => setRiskFilter(e.target.value)}
              className="h-10 rounded-md border border-border bg-background px-3 py-1 text-xs outline-none focus:border-primary cursor-pointer"
            >
              <option value="all">All levels</option>
              <option value="low">Low</option>
              <option value="medium">Medium</option>
              <option value="high">High</option>
              <option value="restricted">Restricted</option>
            </select>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium text-muted-foreground">Lifecycle:</span>
            <select
              value={lifecycleFilter}
              onChange={(e) => setLifecycleFilter(e.target.value)}
              className="h-10 rounded-md border border-border bg-background px-3 py-1 text-xs outline-none focus:border-primary cursor-pointer"
            >
              <option value="all">All states</option>
              <option value="proposed">Proposed</option>
              <option value="active">Active</option>
              <option value="deprecated">Deprecated</option>
              <option value="retired">Retired</option>
            </select>
          </div>
        </div>
      </div>

      {/* Scopes Grid List */}
      <div className="grid gap-4">
        {filteredScopes.map((scope) => (
          <article
            key={scope.scope_key}
            className="group relative rounded-lg border border-border bg-card p-5 transition-all hover:border-primary/30 hover:shadow-md"
          >
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div className="space-y-1">
                <h3 className="text-base font-semibold group-hover:text-primary transition-colors">
                  {scope.display_name}
                </h3>
                <code className="block rounded bg-muted/65 px-2 py-0.5 font-mono text-xs font-medium text-muted-foreground w-fit">
                  {scope.scope_key}
                </code>
              </div>
              <div className="flex items-center gap-2">
                <span
                  className={`rounded-md border px-2 py-1 text-xs font-semibold capitalize tracking-wide ${getRiskBadgeStyles(
                    scope.risk,
                  )}`}
                >
                  {scope.risk} risk
                </span>
                <span
                  className={`rounded-md border px-2 py-1 text-xs font-semibold capitalize tracking-wide ${getLifecycleBadgeStyles(
                    scope.lifecycle,
                  )}`}
                >
                  {scope.lifecycle}
                </span>

                {canManage && (
                  <div className="ml-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                    <button
                      type="button"
                      onClick={() => {
                        setFormError(null);
                        setEditingScope({
                          scope_key: scope.scope_key,
                          display_name: scope.display_name,
                          description: scope.description,
                          risk: scope.risk as ScopeRisk,
                          owner_team: scope.owner_team,
                          lifecycle: scope.lifecycle as ScopeLifecycle,
                          allowed_audiences: scope.allowed_audiences.join(', '),
                        });
                        setEditingScopeKey(scope.scope_key);
                      }}
                      className="rounded p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
                      title="Edit scope"
                    >
                      <Edit2 className="h-4 w-4" />
                    </button>
                    <button
                      type="button"
                      onClick={() => setDeletingScopeKey(scope.scope_key)}
                      className="rounded p-1.5 text-muted-foreground hover:bg-red-500/10 hover:text-red-500 transition-colors"
                      title="Delete scope"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  </div>
                )}
              </div>
            </div>

            <p className="mt-3 text-sm text-muted-foreground leading-relaxed">
              {scope.description}
            </p>

            <div className="mt-4 flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-border/40 pt-3 text-xs text-muted-foreground">
              <span className="flex items-center gap-1.5">
                <Users className="h-3.5 w-3.5 text-muted-foreground/75" />
                <span className="font-semibold text-foreground/80">Owner:</span> {scope.owner_team}
              </span>
              <span className="flex items-center gap-1.5">
                <Globe className="h-3.5 w-3.5 text-muted-foreground/75" />
                <span className="font-semibold text-foreground/80">Audiences:</span>{' '}
                {scope.allowed_audiences.join(', ') || 'Any'}
              </span>
            </div>
          </article>
        ))}

        {filteredScopes.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-lg border border-border bg-card p-12 text-center">
            <Library className="h-10 w-10 text-muted-foreground/60" />
            <h3 className="mt-4 text-base font-semibold">No scopes match filters</h3>
            <p className="mt-2 text-sm text-muted-foreground max-w-sm">
              Adjust your search query or clear filters to view registered scopes.
            </p>
          </div>
        ) : null}
      </div>

      {/* CREATE SCOPE MODAL */}
      {isCreateOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4 animate-in fade-in duration-200">
          <div className="relative w-full max-w-lg rounded-lg border border-border bg-card p-6 shadow-xl animate-in zoom-in-95 duration-200">
            <button
              type="button"
              onClick={() => setIsCreateOpen(false)}
              className="absolute right-4 top-4 rounded-sm opacity-70 hover:opacity-100 focus:outline-none"
            >
              <X className="h-4 w-4" />
            </button>
            <h3 className="text-lg font-semibold">Register scope</h3>
            <p className="text-xs text-muted-foreground mt-1">
              Add a new OAuth scope to the central registry.
            </p>

            {formError && (
              <div className="mt-4 flex items-center gap-2 rounded-md bg-red-500/10 border border-red-500/20 p-3 text-sm text-red-500">
                <AlertTriangle className="h-4 w-4 shrink-0" />
                <span>{formError}</span>
              </div>
            )}

            <form className="mt-4 space-y-4" onSubmit={handleCreateSubmit}>
              <Field label="Scope Key">
                <input
                  name="scope_key"
                  className={inputClass}
                  placeholder="identity.users.read"
                  required
                />
              </Field>
              <Field label="Display Name">
                <input
                  name="display_name"
                  className={inputClass}
                  placeholder="Read user profiles"
                  required
                />
              </Field>
              <Field label="Description">
                <textarea
                  name="description"
                  className={textareaClass}
                  placeholder="Explain what access this scope grants to applications..."
                  required
                />
              </Field>
              <div className="grid grid-cols-2 gap-4">
                <Field label="Risk Level">
                  <select
                    name="risk"
                    defaultValue="medium"
                    className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary w-full cursor-pointer"
                  >
                    <option value="low">Low Risk</option>
                    <option value="medium">Medium Risk</option>
                    <option value="high">High Risk</option>
                    <option value="restricted">Restricted Risk</option>
                  </select>
                </Field>
                <Field label="Lifecycle">
                  <select
                    name="lifecycle"
                    defaultValue="proposed"
                    className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary w-full cursor-pointer"
                  >
                    <option value="proposed">Proposed</option>
                    <option value="active">Active</option>
                    <option value="deprecated">Deprecated</option>
                    <option value="retired">Retired</option>
                  </select>
                </Field>
              </div>
              <Field label="Owner Team">
                <input
                  name="owner_team"
                  className={inputClass}
                  placeholder="Core Identity Team"
                  required
                />
              </Field>
              <Field label="Allowed Audiences (Comma separated)">
                <input
                  name="allowed_audiences"
                  className={inputClass}
                  placeholder="https://api.nvbes.com, optional"
                />
              </Field>

              <div className="mt-6 flex justify-end gap-3 pt-3 border-t border-border/40">
                <button
                  type="button"
                  onClick={() => setIsCreateOpen(false)}
                  className="h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-muted"
                >
                  Cancel
                </button>
                <button type="submit" disabled={createMutation.isPending} className={buttonClass}>
                  {createMutation.isPending ? 'Registering...' : 'Register scope'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* EDIT SCOPE MODAL */}
      {editingScope && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4 animate-in fade-in duration-200">
          <div className="relative w-full max-w-lg rounded-lg border border-border bg-card p-6 shadow-xl animate-in zoom-in-95 duration-200">
            <button
              type="button"
              onClick={() => {
                setEditingScope(null);
                setEditingScopeKey(null);
              }}
              className="absolute right-4 top-4 rounded-sm opacity-70 hover:opacity-100 focus:outline-none"
            >
              <X className="h-4 w-4" />
            </button>
            <h3 className="text-lg font-semibold">Edit scope</h3>
            <p className="text-xs text-muted-foreground mt-1">
              Update scope settings and metadata.
            </p>

            {formError && (
              <div className="mt-4 flex items-center gap-2 rounded-md bg-red-500/10 border border-red-500/20 p-3 text-sm text-red-500">
                <AlertTriangle className="h-4 w-4 shrink-0" />
                <span>{formError}</span>
              </div>
            )}

            <form className="mt-4 space-y-4" onSubmit={handleUpdateSubmit}>
              <Field label="Scope Key">
                <input
                  name="scope_key"
                  className={`${inputClass} disabled:opacity-60 disabled:cursor-not-allowed`}
                  value={editingScope.scope_key}
                  disabled
                />
              </Field>
              <Field label="Display Name">
                <input
                  name="display_name"
                  defaultValue={editingScope.display_name}
                  className={inputClass}
                  required
                />
              </Field>
              <Field label="Description">
                <textarea
                  name="description"
                  defaultValue={editingScope.description}
                  className={textareaClass}
                  required
                />
              </Field>
              <div className="grid grid-cols-2 gap-4">
                <Field label="Risk Level">
                  <select
                    name="risk"
                    defaultValue={editingScope.risk}
                    className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary w-full cursor-pointer"
                  >
                    <option value="low">Low Risk</option>
                    <option value="medium">Medium Risk</option>
                    <option value="high">High Risk</option>
                    <option value="restricted">Restricted Risk</option>
                  </select>
                </Field>
                <Field label="Lifecycle">
                  <select
                    name="lifecycle"
                    defaultValue={editingScope.lifecycle}
                    className="h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary w-full cursor-pointer"
                  >
                    <option value="proposed">Proposed</option>
                    <option value="active">Active</option>
                    <option value="deprecated">Deprecated</option>
                    <option value="retired">Retired</option>
                  </select>
                </Field>
              </div>
              <Field label="Owner Team">
                <input
                  name="owner_team"
                  defaultValue={editingScope.owner_team}
                  className={inputClass}
                  required
                />
              </Field>
              <Field label="Allowed Audiences (Comma separated)">
                <input
                  name="allowed_audiences"
                  defaultValue={editingScope.allowed_audiences}
                  className={inputClass}
                  placeholder="https://api.nvbes.com, optional"
                />
              </Field>

              <div className="mt-6 flex justify-end gap-3 pt-3 border-t border-border/40">
                <button
                  type="button"
                  onClick={() => {
                    setEditingScope(null);
                    setEditingScopeKey(null);
                  }}
                  className="h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-muted"
                >
                  Cancel
                </button>
                <button type="submit" disabled={updateMutation.isPending} className={buttonClass}>
                  {updateMutation.isPending ? 'Saving...' : 'Save changes'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* DELETE SCOPE CONFIRMATION MODAL */}
      {deletingScopeKey && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4 animate-in fade-in duration-200">
          <div className="relative w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-xl animate-in zoom-in-95 duration-200">
            <button
              type="button"
              onClick={() => setDeletingScopeKey(null)}
              className="absolute right-4 top-4 rounded-sm opacity-70 hover:opacity-100 focus:outline-none"
            >
              <X className="h-4 w-4" />
            </button>
            <div className="flex items-start gap-3">
              <div className="rounded-full bg-red-500/10 p-2 text-red-500">
                <AlertTriangle className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-foreground">Delete scope?</h3>
                <p className="text-sm text-muted-foreground mt-2 leading-relaxed">
                  Are you sure you want to delete{' '}
                  <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs font-semibold text-foreground/80">
                    {deletingScopeKey}
                  </code>
                  ? This will remove the scope from the registry and the runtime authorization sync.
                  This action cannot be undone.
                </p>
              </div>
            </div>

            <div className="mt-6 flex justify-end gap-3 pt-3 border-t border-border/40">
              <button
                type="button"
                onClick={() => setDeletingScopeKey(null)}
                className="h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-muted"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => deleteMutation.mutate(deletingScopeKey)}
                disabled={deleteMutation.isPending}
                className="inline-flex h-10 items-center justify-center rounded-md bg-red-600 px-4 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-50"
              >
                {deleteMutation.isPending ? 'Deleting...' : 'Delete scope'}
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

function ScopesUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Scope registry unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load the central scope registry.
      </p>
    </section>
  );
}
