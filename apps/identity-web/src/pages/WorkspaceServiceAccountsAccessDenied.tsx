import type { AccountWorkspace } from '@nvbes/identity-client';
import { Link } from '@tanstack/react-router';
import { ArrowLeft, ShieldAlert } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function WorkspaceServiceAccountsAccessDenied({
  workspace,
  workspaceId,
}: {
  workspace: AccountWorkspace | null;
  workspaceId: string;
}) {
  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up">
      <Card className="overflow-hidden border-destructive/20 bg-gradient-to-br from-destructive/8 via-background to-muted/40">
        <CardHeader className="space-y-4">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="secondary">Surface admin</Badge>
            {workspace ? <Badge variant="outline">{workspace.role}</Badge> : null}
          </div>
          <div className="flex items-start gap-3">
            <div className="flex size-11 shrink-0 items-center justify-center rounded-2xl bg-destructive/10 text-destructive">
              <ShieldAlert className="size-5" />
            </div>
            <div className="min-w-0">
              <CardTitle>Acces restreint aux comptes de service</CardTitle>
              <CardDescription className="mt-1 max-w-2xl">
                La gestion des principaux machine est reservee aux roles{' '}
                <span className="font-medium">owner</span> et{' '}
                <span className="font-medium">admin</span> du workspace courant. Le workspace
                selectionne ne dispose pas du niveau de permission requis.
              </CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Alert>
            <AlertDescription>
              {workspace ? (
                <>
                  Workspace courant:{' '}
                  <span className="font-medium text-foreground">{workspace.name}</span>{' '}
                  <span className="text-muted-foreground">({workspaceId})</span>
                  <br />
                  Role detecte:{' '}
                  <span className="font-medium text-foreground">{workspace.role}</span>
                </>
              ) : (
                <>
                  Workspace courant introuvable.
                  <br />
                  Verifiez que la session contient bien un workspace actif.
                </>
              )}
            </AlertDescription>
          </Alert>
          <div className="flex flex-wrap gap-2">
            <Button asChild>
              <Link to="/account/workspaces">
                <ArrowLeft data-icon="inline-start" />
                Retour aux workspaces
              </Link>
            </Button>
            <Button variant="outline" asChild>
              <Link to="/account">Aller au compte</Link>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
