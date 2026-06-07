import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
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
    <>
      {factors.map((factor, index) => {
        const Icon = getFactorTypeIcon(factor.factor_type);
        const iconClass =
          factor.factor_type === 'recovery'
            ? 'size-4 text-muted-foreground'
            : 'size-4 text-primary';

        return (
          <Card
            key={factor.id}
            className="animate-fade-slide-up"
            style={{ animationDelay: `${(index + 1) * 100}ms` }}
          >
            <CardHeader>
              <div className="flex items-start gap-3">
                <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted">
                  <Icon className={iconClass} />
                </div>
                <div className="flex min-w-0 flex-1 flex-col gap-1">
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
            </CardHeader>
            <CardFooter>
              <Button
                variant="outline"
                size="sm"
                disabled={removingId === factor.id}
                onClick={() => factor.id && onRemove(factor.id)}
              >
                <Trash2 data-icon="inline-start" />
                {removingId === factor.id ? 'Suppression...' : 'Supprimer'}
              </Button>
            </CardFooter>
          </Card>
        );
      })}
    </>
  );
}
