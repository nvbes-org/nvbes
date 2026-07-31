import { Laptop } from 'lucide-react';
import { IdentityEmptyState } from '@/components/IdentityEmptyState';

export function EmptySessionsCard() {
  return (
    <IdentityEmptyState
      icon={Laptop}
      title="Aucune session"
      description="Impossible de charger vos sessions."
      className="border"
    />
  );
}
