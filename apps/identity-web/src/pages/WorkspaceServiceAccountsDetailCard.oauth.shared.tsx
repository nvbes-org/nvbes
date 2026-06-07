import { KeyRound } from 'lucide-react';

export function EmptyClientState() {
  return (
    <div className="flex flex-col items-center gap-3 rounded-2xl border border-dashed border-border/70 px-6 py-10 text-center">
      <div className="flex size-12 items-center justify-center rounded-full bg-muted">
        <KeyRound className="size-5 text-muted-foreground" />
      </div>
      <div className="flex flex-col gap-1">
        <p className="text-sm font-medium">Aucun client OAuth</p>
        <p className="text-sm text-muted-foreground">
          Creer ou attacher un client pour emettre des tokens M2M.
        </p>
      </div>
    </div>
  );
}
