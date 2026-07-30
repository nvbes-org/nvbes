import { Laptop } from 'lucide-react';
import { AccountEmptyState } from '@/components/AccountEmptyState';

export function EmptySessionsCard() {
  return (
    <AccountEmptyState
      icon={Laptop}
      title="Aucune session"
      description="Impossible de charger vos sessions."
      className="border"
    />
  );
}
