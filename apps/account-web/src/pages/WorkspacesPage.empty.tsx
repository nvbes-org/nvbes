import { Building, Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function EmptyWorkspaceState({ onOpenCreate }: { onOpenCreate: () => void }) {
  return (
    <div className="px-6 py-8">
      <div className="flex flex-col items-center gap-3">
        <Building className="size-8 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">Aucun workspace trouve.</p>
        <Button variant="outline" size="sm" onClick={onOpenCreate}>
          <Plus className="size-3.5" data-icon="inline-start" />
          Creer un workspace
        </Button>
      </div>
    </div>
  );
}
