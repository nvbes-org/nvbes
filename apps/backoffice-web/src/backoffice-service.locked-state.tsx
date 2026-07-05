import { LockKeyhole } from 'lucide-react';

export function LockedState({ label = 'Contexte operateur requis' }: { label?: string }) {
  return (
    <div className="border-border bg-muted/40 text-muted-foreground flex items-center gap-3 rounded-md border border-dashed p-3 text-sm">
      <div className="bg-background flex size-8 shrink-0 items-center justify-center rounded-md border">
        <LockKeyhole className="size-4" />
      </div>
      <p>{label}</p>
    </div>
  );
}
