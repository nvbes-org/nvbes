import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { FACTOR_ADD_ACTIONS } from './MfaPage.shared';
import { SecurityActionRow } from '@/pages/AccountSecurityPage.row';
import { ChevronRight } from 'lucide-react';

export function MfaPageAddFactorCard({ onNavigate }: { onNavigate: (path: string) => void }) {
  return (
    <div className="animate-fade-slide-up [animation-delay:300ms]">
      <Card>
        <CardContent className="flex flex-col">
          {FACTOR_ADD_ACTIONS.map((action) => {
            if (action.path === '/account/mfa/recovery-codes') {
              return null;
            }
            return (
              <SecurityActionRow
                icon={action.icon}
                iconColor="size-4 text-muted-foreground"
                label={action.label}
                onAction={() => onNavigate(action.path)}
              />
            );
          })}
        </CardContent>
      </Card>
    </div>
  );
}

export function MfaPageRecoveryCodesCard({
  hasRecovery,
  recoveryCreatedAt,
  onNavigate,
}: {
  hasRecovery: boolean;
  recoveryCreatedAt?: string;
  onNavigate: (path: string) => void;
}) {
  const formattedDate = recoveryCreatedAt
    ? new Date(recoveryCreatedAt).toLocaleDateString('fr-FR')
    : null;

  return (
    <Card className="animate-fade-slide-up [animation-delay:350ms]">
      <CardContent>
        <Button
          variant={hasRecovery ? 'ghost' : 'default'}
          className="h-auto w-full cursor-pointer justify-between gap-3 whitespace-normal rounded-lg px-2 py-3 text-left disabled:cursor-not-allowed"
          onClick={() => onNavigate('/account/mfa/recovery-codes')}
        >
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex min-w-0 flex-col">
              <span className="text-sm font-medium">
                {hasRecovery ? 'Régénérer les codes' : 'Générer les codes'}
              </span>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {hasRecovery && (
              <p className="shrink-0 text-sm">
                {formattedDate ? `généré le ${formattedDate}` : 'généré'}
              </p>
            )}
            <ChevronRight className="size-4" />
          </div>
        </Button>
      </CardContent>
    </Card>
  );
}
