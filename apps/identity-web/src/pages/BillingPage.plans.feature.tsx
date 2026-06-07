import { Check } from 'lucide-react';

export function BillingPlanFeature({ feature }: { feature: string }) {
  return (
    <div className="flex items-center gap-2">
      <Check className="size-3.5 shrink-0 text-primary" />
      <span className="text-sm text-muted-foreground">{feature}</span>
    </div>
  );
}
