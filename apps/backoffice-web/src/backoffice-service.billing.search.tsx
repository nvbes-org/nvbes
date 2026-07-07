import { ClipboardButton } from '@nvbes/web-ui';
import { useQuery } from '@tanstack/react-query';
import { Loader2, Search } from 'lucide-react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { searchBilling } from './backoffice-service.api';
import { LockedState } from './backoffice-service.locked-state';
import type { AdminCredentials } from './backoffice-service.types';

export function BillingSearch({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const [query, setQuery] = useState('');
  const [submitted, setSubmitted] = useState('');
  const trimmedQuery = query.trim();
  const canSearch = !disabled && trimmedQuery.length >= 2;
  const search = useQuery({
    queryKey: ['billing-search', credentials.workspaceId, submitted],
    queryFn: () => searchBilling(credentials, submitted),
    enabled: !disabled && submitted.trim().length >= 2,
  });

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="recherche">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold">Recherche billing</h2>
          <p className="text-muted-foreground text-xs">
            Invoices et payments du workspace courant.
          </p>
        </div>
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
          placeholder="invoice id, payment id, numero..."
          value={query}
        />
        <Button disabled={!canSearch || search.isFetching} type="submit">
          {search.isFetching ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <Search className="size-4" />
          )}
          Chercher
        </Button>
      </form>
      <p className="text-muted-foreground mt-2 text-xs">
        Minimum 2 caracteres. La recherche couvre les factures et paiements du tenant.
      </p>
      <div className="mt-4 overflow-hidden rounded-md border">
        {disabled ? (
          <div className="p-3">
            <LockedState label="Renseigne le workspace, l'acteur et le token interne pour lancer une recherche." />
          </div>
        ) : (
          <SearchResults
            error={search.error}
            hasSubmitted={submitted.length >= 2}
            isFetching={search.isFetching}
            queryTooShort={trimmedQuery.length > 0 && trimmedQuery.length < 2}
            rows={search.data ?? []}
          />
        )}
      </div>
    </section>
  );
}

function SearchResults({
  error,
  hasSubmitted,
  isFetching,
  queryTooShort,
  rows,
}: {
  error: Error | null;
  hasSubmitted: boolean;
  isFetching: boolean;
  queryTooShort: boolean;
  rows: Array<{ id: string; kind: string; label: string; status: string }>;
}) {
  if (queryTooShort) {
    return <SearchMessage title="Requete trop courte" body="Ajoute au moins un caractere." />;
  }
  if (!hasSubmitted) {
    return (
      <SearchMessage
        title="Pret a chercher"
        body="Saisis un identifiant ou un numero de facture."
      />
    );
  }
  if (error) {
    return <SearchMessage title="Recherche indisponible" body={error.message} tone="danger" />;
  }
  if (isFetching && rows.length === 0) {
    return <SearchMessage title="Recherche en cours" body="Chargement des resultats billing..." />;
  }
  if (rows.length === 0) {
    return (
      <SearchMessage
        title="Aucun resultat"
        body="Aucune facture ou paiement ne correspond a cette recherche."
      />
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Type</TableHead>
          <TableHead>Label</TableHead>
          <TableHead>Status</TableHead>
          <TableHead>ID</TableHead>
          <TableHead className="w-12" />
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={`${row.kind}:${row.id}`}>
            <TableCell>
              <Badge variant="secondary">{row.kind}</Badge>
            </TableCell>
            <TableCell className="font-medium">{row.label}</TableCell>
            <TableCell>{row.status}</TableCell>
            <TableCell className="max-w-[220px] truncate font-mono text-xs">{row.id}</TableCell>
            <TableCell>
              <ClipboardButton
                className="size-7 rounded-[min(var(--radius-md),12px)] border-transparent bg-transparent p-0 hover:bg-muted"
                label={`Copy ${row.kind} ID`}
                value={row.id}
              />
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
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
    <div className="flex min-h-28 items-center justify-center p-4 text-center">
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
