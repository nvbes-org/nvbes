import { ChevronRight } from 'lucide-react';
import { IdentityActionItem } from '@/components/IdentityActionItem';
import { Card, CardContent } from '@/components/ui/card';
import { FACTOR_ADD_ACTIONS } from './MfaPage.shared';

export function MfaPageAddFactorCard({ onNavigate }: { onNavigate: (path: string) => void }) {
  return (
    <div className="animate-fade-slide-up [animation-delay:300ms]">
      <Card>
        <CardContent className="flex flex-col">
          {FACTOR_ADD_ACTIONS.map((action) => {
            if (action.path === '/mfa/recovery-codes') {
              return null;
            }
            return (
              <IdentityActionItem
                key={action.path}
                icon={action.icon}
                iconClassName="size-4 text-muted-foreground"
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
        <IdentityActionItem
          variant={hasRecovery ? 'ghost' : 'default'}
          label={hasRecovery ? 'Régénérer les codes' : 'Générer les codes'}
          trailing={
            <>
              {hasRecovery ? (
                <p className="shrink-0 text-sm">
                  {formattedDate ? `généré le ${formattedDate}` : 'généré'}
                </p>
              ) : null}
              <ChevronRight className="size-4" />
            </>
          }
          onAction={() => onNavigate('/mfa/recovery-codes')}
        />
      </CardContent>
    </Card>
  );
}
