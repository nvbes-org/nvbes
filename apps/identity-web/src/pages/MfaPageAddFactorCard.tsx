import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { FACTOR_ADD_ACTIONS } from './MfaPage.shared';

export function MfaPageAddFactorCard({
  hasRecovery,
  onNavigate,
}: {
  hasRecovery: boolean;
  onNavigate: (path: string) => void;
}) {
  return (
    <Card className="animate-fade-slide-up [animation-delay:300ms]">
      <CardHeader>
        <CardTitle>Ajouter une méthode</CardTitle>
        <CardDescription>Choisissez une méthode d'authentification supplémentaire.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        {FACTOR_ADD_ACTIONS.map((action) => {
          if (action.path === '/account/mfa/recovery-codes' && hasRecovery) {
            return null;
          }
          return (
            <Button
              key={action.path}
              variant="outline"
              className="justify-start gap-3"
              onClick={() => onNavigate(action.path)}
            >
              <action.icon className="size-4 text-muted-foreground" />
              {action.label}
            </Button>
          );
        })}
      </CardContent>
    </Card>
  );
}
