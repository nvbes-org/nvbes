import { Zap } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { cn } from '@/lib/classnames';
import { BillingPlanFeature } from './BillingPage.plans.feature';
import type { Plan } from './BillingPage.plans.data';

export function BillingPlanCard({
  plan,
  isCurrent,
  checkoutLoading,
  onCheckout,
}: {
  plan: Plan;
  isCurrent: boolean;
  checkoutLoading: string | null;
  onCheckout: (planCode: string) => void;
}) {
  return (
    <Card
      className={cn(
        plan.featured && !isCurrent && 'ring-2 ring-primary/30',
        isCurrent && 'opacity-60',
      )}
    >
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>{plan.name}</CardTitle>
          <div className="flex items-center gap-2">
            {isCurrent ? (
              <Badge variant="secondary" className="shrink-0">
                Plan actuel
              </Badge>
            ) : null}
            {plan.featured && !isCurrent ? (
              <Badge variant="default" className="shrink-0">
                <Zap className="size-3" data-icon="inline-start" />
                Populaire
              </Badge>
            ) : null}
          </div>
        </div>
        <CardDescription>
          <span className="text-2xl font-bold text-foreground">{plan.price}€</span>
          <span className="text-sm text-muted-foreground">{plan.period}</span>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-1.5">
          {plan.features.map((feature) => (
            <BillingPlanFeature key={feature} feature={feature} />
          ))}
        </div>
      </CardContent>
      <CardFooter>
        <Button
          variant={plan.featured && !isCurrent ? 'default' : 'outline'}
          className="w-full"
          onClick={() => onCheckout(plan.code)}
          disabled={checkoutLoading === plan.code || isCurrent}
        >
          {isCurrent
            ? 'Abonnement actif'
            : checkoutLoading === plan.code
              ? 'Chargement...'
              : `Choisir ${plan.name}`}
        </Button>
      </CardFooter>
    </Card>
  );
}
