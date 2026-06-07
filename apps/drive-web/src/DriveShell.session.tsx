import { Upload } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { DriveMeResponse } from './drive.api';

function StatRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-2xl border border-border/60 bg-background px-4 py-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="max-w-[14rem] truncate font-medium">{value}</span>
    </div>
  );
}

export function DriveSessionContextCard({
  user,
  currentWorkspace,
}: {
  user: DriveMeResponse['user'];
  currentWorkspace:
    | {
        name: string;
        role?: string | null;
      }
    | undefined;
}) {
  return (
    <Card className="border-border/60 shadow-sm">
      <CardHeader className="space-y-1">
        <CardTitle className="text-base">Contexte de session</CardTitle>
        <CardDescription>Utilisateur et workspace recuperes via `/auth/me`.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 text-sm">
        <StatRow label="Utilisateur" value={user.display_name} />
        <StatRow label="Email" value={user.email} />
        <StatRow label="Workspace" value={currentWorkspace?.name ?? 'Aucun'} />
        <StatRow label="Role" value={currentWorkspace?.role ?? 'n/a'} />
        <div className="rounded-2xl border border-border/60 bg-muted/40 p-4">
          <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.28em] text-muted-foreground">
            <Upload className="size-3.5" />
            Upload natif
          </div>
          <p className="text-sm text-muted-foreground">
            Utilisez le bouton "Ouvrir" pour transferer des fichiers depuis votre appareil via
            l&apos;API File System Access.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
