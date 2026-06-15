import type { AccessReviewItem } from '@nvbes/identity-client';
import { Label } from '../components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';

export type AccessReviewItemTypeFilter = AccessReviewItem['item_type'] | 'all';
export type AccessReviewDecisionFilter = AccessReviewItem['decision'] | 'all';

export type AccessReviewFilters = {
  itemType: AccessReviewItemTypeFilter;
  decision: AccessReviewDecisionFilter;
};

const ItemTypeOptions: AccessReviewItemTypeFilter[] = [
  'all',
  'member',
  'role',
  'service_account',
  'oauth_client',
];

const DecisionOptions: AccessReviewDecisionFilter[] = [
  'all',
  'pending',
  'approved',
  'revoked',
  'changed',
];

export function AccessReviewFilterControls({
  filters,
  onChange,
}: {
  filters: AccessReviewFilters;
  onChange: (filters: AccessReviewFilters) => void;
}) {
  return (
    <div className="mb-4 grid gap-3 sm:grid-cols-2 lg:max-w-xl">
      <div className="grid gap-2">
        <Label htmlFor="access-review-item-type">Item type</Label>
        <Select
          value={filters.itemType}
          onValueChange={(itemType) =>
            onChange({
              ...filters,
              itemType: itemType as AccessReviewItemTypeFilter,
            })
          }
        >
          <SelectTrigger id="access-review-item-type">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {ItemTypeOptions.map((itemType) => (
              <SelectItem key={itemType} value={itemType}>
                {formatOption(itemType)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="grid gap-2">
        <Label htmlFor="access-review-decision">Decision</Label>
        <Select
          value={filters.decision}
          onValueChange={(decision) =>
            onChange({
              ...filters,
              decision: decision as AccessReviewDecisionFilter,
            })
          }
        >
          <SelectTrigger id="access-review-decision">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {DecisionOptions.map((decision) => (
              <SelectItem key={decision} value={decision}>
                {formatOption(decision)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}

export function filterAccessReviewItems(items: AccessReviewItem[], filters: AccessReviewFilters) {
  return items.filter((item) => {
    const typeMatches = filters.itemType === 'all' || item.item_type === filters.itemType;
    const decisionMatches = filters.decision === 'all' || item.decision === filters.decision;
    return typeMatches && decisionMatches;
  });
}

function formatOption(value: string) {
  return value
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}
