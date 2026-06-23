import { ClipboardButton } from '@nvbes/web-ui';
import { useQuery } from '@tanstack/react-query';
import { Loader2, Search } from 'lucide-react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { globalSearch } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials, GlobalSearchResult } from './internal-admin.types';

export function GlobalSearchPanel({
  credentials,
  disabled,
  onSelectTenant,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
}) {
  const [query, setQuery] = useState('');
  const [submitted, setSubmitted] = useState('');
  const trimmedQuery = query.trim();
  const canSearch = !disabled && trimmedQuery.length >= 2;
  const search = useQuery({
    queryKey: ['global-search', submitted],
    queryFn: () => globalSearch(credentials, submitted),
    enabled: submitted.length >= 2 && !disabled,
  });

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="global-search">
      <div className="mb-4">
        <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
          Global search
        </p>
        <h2 className="text-base font-semibold">Tenants, workspaces, users</h2>
      </div>
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (canSearch) setSubmitted(trimmedQuery);
        }}
      >
        <Input
          disabled={disabled}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="tenant slug, workspace, email, UUID..."
          value={query}
        />
        <Button disabled={!canSearch || search.isFetching} type="submit">
          {search.isFetching ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <Search className="size-4" />
          )}
          Search
        </Button>
      </form>
      <div className="mt-4">
        {disabled ? (
          <LockedState label="Connecte un contexte operateur pour utiliser la recherche globale." />
        ) : (
          <GlobalSearchResults
            error={search.error}
            hasSubmitted={submitted.length >= 2}
            isFetching={search.isFetching}
            onSelectTenant={onSelectTenant}
            rows={search.data ?? []}
            tooShort={trimmedQuery.length > 0 && trimmedQuery.length < 2}
          />
        )}
      </div>
    </section>
  );
}

function GlobalSearchResults({
  error,
  hasSubmitted,
  isFetching,
  onSelectTenant,
  rows,
  tooShort,
}: {
  error: Error | null;
  hasSubmitted: boolean;
  isFetching: boolean;
  onSelectTenant: (tenantId: string) => void;
  rows: GlobalSearchResult[];
  tooShort: boolean;
}) {
  if (tooShort) return <SearchMessage title="Requete trop courte" body="Minimum 2 caracteres." />;
  if (!hasSubmitted) {
    return (
      <SearchMessage title="Pret a chercher" body="Recherche transverse sur les entites coeur." />
    );
  }
  if (error)
    return <SearchMessage title="Recherche indisponible" body={error.message} tone="danger" />;
  if (isFetching && rows.length === 0) {
    return <SearchMessage title="Recherche en cours" body="Chargement des entites..." />;
  }
  if (rows.length === 0) {
    return <SearchMessage title="Aucun resultat" body="Aucun tenant, workspace ou user trouve." />;
  }

  return (
    <div className="grid gap-2">
      {rows.map((row) => (
        <article className="bg-muted/40 rounded-md border p-3" key={`${row.kind}:${row.id}`}>
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="mb-1 flex flex-wrap items-center gap-2">
                <Badge variant="secondary">{row.kind}</Badge>
                <Badge variant="outline">{row.status}</Badge>
              </div>
              <h3 className="truncate text-sm font-medium">{row.label}</h3>
              <p className="text-muted-foreground mt-1 truncate font-mono text-xs">{row.id}</p>
            </div>
            <ClipboardButton
              className="size-7 rounded-[min(var(--radius-md),12px)] border-transparent bg-transparent p-0 hover:bg-muted"
              label={`Copy ${row.kind} ID`}
              value={row.id}
            />
            {row.tenant_id ? (
              <Button
                onClick={() => {
                  onSelectTenant(row.tenant_id ?? row.id);
                  window.location.hash = 'tenant-detail';
                }}
                size="sm"
                type="button"
              >
                Open tenant
              </Button>
            ) : null}
          </div>
        </article>
      ))}
    </div>
  );
}

function SearchMessage({
  body,
  title,
  tone = 'default',
}: {
  body: string;
  title: string;
  tone?: 'danger' | 'default';
}) {
  return (
    <div className="bg-muted/30 flex min-h-24 items-center justify-center rounded-md border p-4 text-center">
      <div>
        <p
          className={
            tone === 'danger' ? 'text-destructive text-sm font-medium' : 'text-sm font-medium'
          }
        >
          {title}
        </p>
        <p className="text-muted-foreground mt-1 text-xs">{body}</p>
      </div>
    </div>
  );
}
