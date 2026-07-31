import { Unlink } from 'lucide-react';
import { IdentityEmptyState } from '@/components/IdentityEmptyState';

export function LinkedAppsEmptyState() {
  return (
    <IdentityEmptyState
      icon={Unlink}
      title="Aucune application liée"
      description="Aucune application tierce autorisee."
      className="px-6 py-8"
    />
  );
}
