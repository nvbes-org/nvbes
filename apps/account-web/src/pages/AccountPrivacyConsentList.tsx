import type { UserConsent } from '@nvbes/identity-client';
import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { ItemSeparator } from '@/components/ui/item';
import {
  ConsentEmptyState,
  ConsentRow,
  consentLabels,
  isVisibleConsentType,
} from './AccountPrivacyPage.consents';

export function AccountPrivacyConsentList({
  consents,
  onRevoke,
}: {
  consents: UserConsent[];
  onRevoke: (consent: UserConsent) => void;
}) {
  const visibleConsents = consents.filter((consent) => isVisibleConsentType(consent.consent_type));

  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2>Historique de vos choix</h2>
        </CardTitle>
        <CardDescription>
          Les accords enregistrés sur votre compte. Chaque choix révocable peut être retiré ici.
        </CardDescription>
        <CardAction>
          <Badge variant="outline">
            {visibleConsents.length} {visibleConsents.length === 1 ? 'actif' : 'actifs'}
          </Badge>
        </CardAction>
      </CardHeader>
      <CardContent>
        {visibleConsents.length === 0 ? (
          <ConsentEmptyState />
        ) : (
          <>
            {visibleConsents.map((consent, index) => {
              const label = consentLabels[consent.consent_type] ?? consent.consent_type;

              return (
                <div key={consent.id}>
                  {index > 0 && <ItemSeparator />}
                  <ConsentRow consent={consent} label={label} onRevoke={() => onRevoke(consent)} />
                </div>
              );
            })}
          </>
        )}
      </CardContent>
    </Card>
  );
}
