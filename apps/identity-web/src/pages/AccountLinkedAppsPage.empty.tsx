import { Unlink } from 'lucide-react';

export function LinkedAppsEmptyState() {
  return (
    <div className="px-6 py-8">
      <div className="flex flex-col items-center gap-3">
        <Unlink className="size-8 text-muted-foreground" />
        <div className="text-center">
          <p className="text-sm text-muted-foreground">Aucune application tierce autorisee.</p>
        </div>
      </div>
    </div>
  );
}
