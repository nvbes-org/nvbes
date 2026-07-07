import { BillingPlanCard } from './BillingPage.plans.card';
import { availablePlans } from './BillingPage.plans.data';

export { availablePlans, type Plan } from './BillingPage.plans.data';

export function BillingPlansList({
  currentPlanCode,
  checkoutLoading,
  onCheckout,
}: {
  currentPlanCode: string;
  checkoutLoading: string | null;
  onCheckout: (planCode: string) => void;
}) {
  return (
    <div>
      <h2 className="mb-3 text-lg font-heading font-semibold">Plans disponibles</h2>
      <div className="flex flex-col gap-4">
        {availablePlans.map((plan) => (
          <BillingPlanCard
            key={plan.code}
            plan={plan}
            isCurrent={plan.code === currentPlanCode}
            checkoutLoading={checkoutLoading}
            onCheckout={onCheckout}
          />
        ))}
      </div>
    </div>
  );
}
