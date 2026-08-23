import type {
  AccessReviewCampaignSummary,
  AccessReviewDecisionInput,
  AccessReviewItem,
} from '@nvbes/identity-client';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';

export function CampaignTable({
  campaigns,
  selectedCampaignId,
  onSelect,
}: {
  campaigns: AccessReviewCampaignSummary[];
  selectedCampaignId: string | null;
  onSelect: (campaignId: string) => void;
}) {
  if (campaigns.length === 0) {
    return (
      <div className="flex min-h-44 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        No access review campaigns yet.
      </div>
    );
  }
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead>Status</TableHead>
          <TableHead>Due</TableHead>
          <TableHead>Reminders</TableHead>
          <TableHead className="text-right">Pending</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {campaigns.map((campaign) => (
          <TableRow
            key={campaign.id}
            data-state={campaign.id === selectedCampaignId ? 'selected' : undefined}
            onClick={() => onSelect(campaign.id)}
            className="cursor-pointer"
          >
            <TableCell className="font-medium">{campaign.name}</TableCell>
            <TableCell>
              <Badge variant={campaign.status === 'active' ? 'default' : 'outline'}>
                {campaign.status}
              </Badge>
            </TableCell>
            <TableCell>{formatDate(campaign.due_at)}</TableCell>
            <TableCell>
              <ReminderSummary campaign={campaign} />
            </TableCell>
            <TableCell className="text-right">{campaign.pending_items}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function ReminderSummary({ campaign }: { campaign: AccessReviewCampaignSummary }) {
  const total = campaign.due_soon_reminders_sent + campaign.overdue_reminders_sent;
  if (total === 0) {
    return <span className="text-xs text-muted-foreground">none</span>;
  }
  return (
    <div className="flex flex-wrap gap-1">
      {campaign.due_soon_reminders_sent > 0 ? (
        <Badge variant="outline" className="rounded-md font-normal">
          due {campaign.due_soon_reminders_sent}
        </Badge>
      ) : null}
      {campaign.overdue_reminders_sent > 0 ? (
        <Badge variant="destructive" className="rounded-md font-normal">
          overdue {campaign.overdue_reminders_sent}
        </Badge>
      ) : null}
    </div>
  );
}

export function ReviewItemsTable({
  items,
  disabled,
  noteReady,
  targetRole,
  onDecide,
}: {
  items: AccessReviewItem[];
  disabled: boolean;
  noteReady: boolean;
  targetRole: string;
  onDecide: (itemId: string, decision: AccessReviewDecisionInput['decision']) => void;
}) {
  if (items.length === 0) {
    return (
      <div className="flex min-h-44 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
        Select or create a campaign to inspect review items.
      </div>
    );
  }
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Subject</TableHead>
          <TableHead>Type</TableHead>
          <TableHead>Role</TableHead>
          <TableHead>Status</TableHead>
          <TableHead>Evidence</TableHead>
          <TableHead>Decision</TableHead>
          <TableHead className="text-right">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((item) => (
          <TableRow key={item.id}>
            <TableCell className="font-medium">{item.subject_label}</TableCell>
            <TableCell>{item.item_type}</TableCell>
            <TableCell>{item.role ?? 'none'}</TableCell>
            <TableCell>{item.status}</TableCell>
            <TableCell>
              <EvidenceSummary item={item} />
            </TableCell>
            <TableCell>
              <Badge variant={item.decision === 'pending' ? 'outline' : 'secondary'}>
                {item.decision}
              </Badge>
            </TableCell>
            <TableCell>
              <DecisionActions
                disabled={disabled}
                noteReady={noteReady}
                targetRole={targetRole}
                item={item}
                onDecide={onDecide}
              />
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function EvidenceSummary({ item }: { item: AccessReviewItem }) {
  const evidence = evidencePairs(item);
  if (evidence.length === 0) {
    return <span className="text-xs text-muted-foreground">none</span>;
  }
  return (
    <div className="flex max-w-72 flex-wrap gap-1">
      {evidence.map(([label, value]) => (
        <Badge key={label} variant="outline" className="rounded-md font-normal">
          {label}: {value}
        </Badge>
      ))}
    </div>
  );
}

function evidencePairs(item: AccessReviewItem) {
  const evidence = item.evidence;
  return [
    ['workspace', readEvidenceString(evidence.workspace_name)],
    ['auth', readEvidenceString(evidence.auth_method)],
    ['client', readEvidenceString(evidence.client_type)],
    ['scope', readEvidenceString(evidence.owner_scope_type)],
    ['source', readEvidenceString(evidence.source)],
    ['last used', formatEvidenceDate(evidence.last_used_at)],
  ].filter((entry): entry is [string, string] => Boolean(entry[1]));
}

function readEvidenceString(value: unknown) {
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : null;
}

function formatEvidenceDate(value: unknown) {
  const raw = readEvidenceString(value);
  if (!raw) {
    return null;
  }
  const date = new Date(raw);
  if (Number.isNaN(date.getTime())) {
    return raw;
  }
  return formatDate(raw);
}

function DecisionActions({
  item,
  disabled,
  noteReady,
  targetRole,
  onDecide,
}: {
  item: AccessReviewItem;
  disabled: boolean;
  noteReady: boolean;
  targetRole: string;
  onDecide: (itemId: string, decision: AccessReviewDecisionInput['decision']) => void;
}) {
  if (item.decision !== 'pending') {
    return <div className="flex justify-end text-xs text-muted-foreground">Reviewed</div>;
  }
  const canChange =
    (item.item_type === 'role' || item.item_type === 'service_account') && item.role !== targetRole;
  return (
    <div className="flex justify-end gap-1">
      <Button
        type="button"
        size="xs"
        variant="outline"
        disabled={disabled}
        onClick={() => onDecide(item.id, 'approved')}
      >
        Approve
      </Button>
      <Button
        type="button"
        size="xs"
        variant="outline"
        disabled={disabled || !noteReady || !canChange}
        onClick={() => onDecide(item.id, 'changed')}
      >
        Change
      </Button>
      <Button
        type="button"
        size="xs"
        variant="destructive"
        disabled={disabled || !noteReady}
        onClick={() => onDecide(item.id, 'revoked')}
      >
        Revoke
      </Button>
    </div>
  );
}

function formatDate(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  }).format(new Date(value));
}
