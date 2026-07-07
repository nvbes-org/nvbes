import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { factorLabel, getFactorTypeIcon } from './MfaPage.shared';

export function MfaPageFactorList({
  factors,
  removingId,
  onRemove,
}: {
  factors: MfaFactorView[];
  removingId: string | null;
  onRemove: (factorId: string) => void;
}) {
  return (
    <div className="flex flex-col gap-2.5">
      {factors.map((factor, index) => {
        const iconType =
          factor.factor_type === 'webauthn' && factor.kind === 'passkey'
            ? 'passkey'
            : factor.factor_type;
        const Icon = getFactorTypeIcon(iconType);
        const iconClass =
          factor.factor_type === 'recovery'
            ? 'size-5 text-muted-foreground'
            : 'size-5 text-primary';

        return (
          <Card
            key={factor.id}
            className="animate-fade-slide-up py-0"
            style={{ animationDelay: `${(index + 1) * 100}ms` }}
          >
            <CardHeader className="px-3 py-2 sm:px-4">
              <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                <div className="flex min-w-0 items-center gap-2.5">
                  <div className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted">
                    <Icon className={iconClass} />
                  </div>
                  <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <div className="flex items-center gap-2">
                      <CardTitle className="truncate">
                        {factor.label ?? factorLabel(factor)}
                      </CardTitle>
                      <Badge
                        variant={factor.status === 'active' ? 'default' : 'secondary'}
                        className="shrink-0"
                      >
                        {factor.status === 'active' ? 'Actif' : 'En attente'}
                      </Badge>
                    </div>
                    {factor.last_used_at && (
                      <CardDescription>
                        Dernière utilisation : {new Date(factor.last_used_at).toLocaleDateString()}
                      </CardDescription>
                    )}
                  </div>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full shrink-0 sm:w-auto"
                  disabled={removingId === factor.id}
                  onClick={() => factor.id && onRemove(factor.id)}
                >
                  <Trash2 data-icon="inline-start" />
                  {removingId === factor.id ? 'Suppression...' : 'Supprimer'}
                </Button>
              </div>
            </CardHeader>
          </Card>
        );
      })}
    </div>
  );
}
