import { X } from 'lucide-react';
import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty';
import type {
  DriveEntry,
  DriveMember,
  DriveShareStatus,
  DriveWorkspaceState,
} from './drive.workspace.types';

const BYTE_FORMATTER = new Intl.NumberFormat('fr-FR', {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});

const DATE_FORMATTER = new Intl.DateTimeFormat('fr-FR', {
  day: '2-digit',
  month: 'short',
  year: 'numeric',
});

export function DriveDetailsPanel({
  state,
  onClose,
}: {
  state: DriveWorkspaceState;
  onClose: () => void;
}) {
  if (state.detailsSelection === null) return null;

  if (state.detailsSelection.type === 'entry') {
    const entry = state.entries.find((candidate) => candidate.id === state.detailsSelection?.id);

    return (
      <DetailsFrame title={entry?.name ?? 'Element introuvable'} onClose={onClose}>
        {entry ? (
          <EntryDetails entry={entry} state={state} />
        ) : (
          <NotFoundDetails label="Ce fichier ou dossier" />
        )}
      </DetailsFrame>
    );
  }

  const member = state.members.find((candidate) => candidate.id === state.detailsSelection?.id);

  return (
    <DetailsFrame title={member?.name ?? 'Membre introuvable'} onClose={onClose}>
      {member ? <MemberDetails member={member} /> : <NotFoundDetails label="Ce membre" />}
    </DetailsFrame>
  );
}

function DetailsFrame({
  title,
  children,
  onClose,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
}) {
  return (
    <section className="grid gap-5" aria-label="Details de la selection">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Details
          </p>
          <h2 className="mt-1 break-words text-lg font-semibold">{title}</h2>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          onClick={onClose}
          aria-label="Fermer les details"
        >
          <X aria-hidden="true" />
        </Button>
      </div>
      {children}
    </section>
  );
}

function EntryDetails({ entry, state }: { entry: DriveEntry; state: DriveWorkspaceState }) {
  const owner = state.members.find((member) => member.id === entry.ownerId);
  const shareLinks = state.shareLinks.filter((link) => link.entryId === entry.id);
  const latestSecurityEvent = state.securityEvents.find((event) => event.target === entry.name);

  return (
    <div className="grid gap-5">
      <DetailsList
        items={[
          ['Statut', entryStatusLabel(entry)],
          ['Partage', sharingLabel(entry, shareLinks.length)],
          ['Proprietaire', ownerLabel(owner)],
          ['Taille', formatBytes(entry.sizeBytes)],
        ]}
      />
      <DetailsSection title="Securite et metadonnees">
        <DetailsList
          items={[
            ['Type', entry.kind],
            ['MIME', entry.mimeType ?? 'Non renseigne'],
            ['Cree le', formatDate(entry.createdAt)],
            ['Modifie le', formatDate(entry.updatedAt)],
            ['Favori', entry.starred ? 'Oui' : 'Non'],
            ['Dernier evenement', latestSecurityEvent?.action ?? 'Aucun evenement recent'],
          ]}
        />
      </DetailsSection>
    </div>
  );
}

function MemberDetails({ member }: { member: DriveMember }) {
  return (
    <DetailsList
      items={[
        ['Email', member.email],
        ['Role', member.role],
        ['Statut', member.status],
        ['Arrivee', member.joinedAt ? formatDate(member.joinedAt) : 'Invitation en attente'],
      ]}
    />
  );
}

function NotFoundDetails({ label }: { label: string }) {
  return (
    <Empty className="border">
      <EmptyHeader>
        <EmptyTitle>Selection indisponible</EmptyTitle>
        <EmptyDescription>
          {label} n'est plus disponible dans l'etat courant du workspace.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}

function DetailsSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-3">
      <h3 className="text-sm font-semibold">{title}</h3>
      {children}
    </section>
  );
}

function DetailsList({ items }: { items: Array<[string, string]> }) {
  return (
    <dl className="grid gap-3 text-sm">
      {items.map(([label, value]) => (
        <Card key={label} className="grid gap-1 p-3">
          <dt className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {label}
          </dt>
          <dd className="break-words font-medium">{value}</dd>
        </Card>
      ))}
    </dl>
  );
}

function entryStatusLabel(entry: DriveEntry): string {
  if (entry.status === 'trashed') return 'Dans la corbeille';
  if (entry.starred) return 'Actif, favori';
  return 'Actif';
}

function sharingLabel(entry: DriveEntry, linkCount: number): string {
  if (entry.shareStatus === 'private') return 'Prive';
  if (entry.shareStatus === 'shared') {
    const linkText =
      linkCount > 0 ? `${linkCount} lien${linkCount > 1 ? 's' : ''}` : 'aucun lien actif';
    return `Partage avec ${entry.sharedWithCount} membre${entry.sharedWithCount > 1 ? 's' : ''}, ${linkText}`;
  }

  return shareStatusLabel(entry.shareStatus);
}

function shareStatusLabel(status: DriveShareStatus): string {
  if (status === 'revoked') return 'Partage desactive';
  if (status === 'expired') return 'Partage expire';
  return status;
}

function ownerLabel(owner: DriveMember | undefined): string {
  if (!owner) return 'Inconnu';
  return `${owner.name} (${owner.email})`;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '-';

  const units = ['o', 'Ko', 'Mo', 'Go', 'To'];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** unitIndex;

  return `${BYTE_FORMATTER.format(value)} ${units[unitIndex]}`;
}

function formatDate(value: string): string {
  return DATE_FORMATTER.format(new Date(value));
}
