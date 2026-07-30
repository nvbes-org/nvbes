import { Unlink } from 'lucide-react';
import { AccountEmptyState } from '@/components/AccountEmptyState';

export function LinkedAppsEmptyState() {
  return (
    <AccountEmptyState
      icon={Unlink}
      title="Aucune application liée"
      description="Aucune application tierce autorisee."
      className="px-6 py-8"
    />
  );
}
