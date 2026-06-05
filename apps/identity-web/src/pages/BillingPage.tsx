import { type BillingOverview, identityClient } from '@nvbes/identity-client';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Check, CreditCard, Package, Zap } from 'lucide-react';
import { useState } from 'react';
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
import { useAccountContext } from '@/hooks/useAccountContext';
import { cn } from '@/lib/utils';
import { trackEvent } from '../identity.posthog';

interface Plan {
  code: string;
  name: string;
  price: string;
  period: string;
  features: string[];
  featured?: boolean;
}

const availablePlans: Plan[] = [
  {
    code: 'solo_pro',
    name: 'Solo Pro',
    price: '15',
    period: '/mois',
    features: ['1 utilisateur', '100 Go de stockage', 'Support par email', 'API Access'],
  },
  {
    code: 'team',
    name: 'Team',
    price: '39',
    period: '/mois',
    features: [
      "Jusqu'a 10 utilisateurs",
      '500 Go de stockage',
      'Support prioritaire',
      'SSO & SAML',
      'Audit logs',
    ],
    featured: true,
  },
  {
    code: 'team_plus',
    name: 'Team Plus',
    price: '79',
    period: '/mois',
    features: [
      'Utilisateurs illimites',
      '2 To de stockage',
      'Support dedie 24/7',
      'SSO, SAML, SCIM',
      'Audit logs avances',
      'SLA 99.9%',
    ],
  },
];

function formatCents(cents: number, currency: string): string {
  return new Intl.NumberFormat('fr-FR', {
    style: 'currency',
    currency: currency.toUpperCase(),
  }).format(cents / 100);
}

function CurrentPlanCard({ overview }: { overview: BillingOverview }) {
  const { plan, subscription } = overview;

  return (
    <Card className="ring-2 ring-primary/30">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>{plan.code}</CardTitle>
          <Badge variant="default" className="shrink-0">
            Plan actuel
          </Badge>
        </div>
        <CardDescription>
          <span className="text-2xl font-bold text-foreground">
            {formatCents(plan.monthly_price_cents, plan.currency)}
          </span>
          <span className="text-sm text-muted-foreground">/mois</span>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center gap-2">
            <Check className="size-3.5 text-primary shrink-0" />
            <span className="text-sm text-muted-foreground">
              {plan.included_users} utilisateur{plan.included_users > 1 ? 's' : ''}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Check className="size-3.5 text-primary shrink-0" />
            <span className="text-sm text-muted-foreground">
              {plan.included_storage_gb} Go de stockage
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Check className="size-3.5 text-primary shrink-0" />
            <span className="text-sm text-muted-foreground">
              Statut : {subscription.status}
              {subscription.trial_ends_at && " (periode d'essai)"}
            </span>
          </div>
        </div>
      </CardContent>
      <CardFooter>
        <Button variant="outline" className="w-full justify-between" disabled>
          Abonnement actif
          <Package className="size-4" data-icon="inline-end" />
        </Button>
      </CardFooter>
    </Card>
  );
}

export default function BillingPage() {
  const { me } = useAccountContext();
  const workspaceId = me?.current_workspace_id;

  if (!workspaceId) {
    return (
      <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
        <div>
          <h1 className="text-xl font-heading font-semibold">Facturation</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Gerer votre abonnement et votre portefeuille.
          </p>
        </div>
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-8">
            <CreditCard className="size-8 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              Vous devez avoir un workspace actif pour acceder a la facturation.
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return <BillingContent workspaceId={workspaceId} />;
}

function BillingContent({ workspaceId }: { workspaceId: string }) {
  const [checkoutLoading, setCheckoutLoading] = useState<string | null>(null);
  const [portalLoading, setPortalLoading] = useState(false);

  const { data: overview } = useSuspenseQuery({
    queryKey: ['billing', 'overview', workspaceId],
    queryFn: () => identityClient.getBillingOverview(workspaceId),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const handleCheckout = async (planCode: string) => {
    if (!workspaceId) return;
    setCheckoutLoading(planCode);
    try {
      trackEvent('billing.checkout_started', {
        plan_code: planCode,
        workspace_id: workspaceId,
      });
      window.location.href = await identityClient.createBillingCheckout(workspaceId, planCode);
    } finally {
      setCheckoutLoading(null);
    }
  };

  const handlePortal = async () => {
    if (!workspaceId) return;
    setPortalLoading(true);
    try {
      window.location.href = await identityClient.createBillingPortal(workspaceId);
    } finally {
      setPortalLoading(false);
    }
  };

  const currentPlanCode = overview.plan.code;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Facturation</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer votre abonnement et votre portefeuille.
        </p>
      </div>

      <CurrentPlanCard overview={overview} />

      <Card>
        <CardHeader>
          <CardTitle>Portail de facturation</CardTitle>
          <CardDescription>
            Accedez a votre portail Stripe pour gerer vos moyens de paiement et factures.
          </CardDescription>
        </CardHeader>
        <CardFooter>
          <Button
            variant="outline"
            className="w-full justify-between"
            onClick={handlePortal}
            disabled={portalLoading}
          >
            {portalLoading ? 'Chargement...' : 'Ouvrir le portail de facturation'}
            <CreditCard data-icon="inline-end" />
          </Button>
        </CardFooter>
      </Card>

      <div>
        <h2 className="text-lg font-heading font-semibold mb-3">Plans disponibles</h2>
        <div className="flex flex-col gap-4">
          {availablePlans.map((plan) => {
            const isCurrent = plan.code === currentPlanCode;
            return (
              <Card
                key={plan.code}
                className={cn(
                  plan.featured && !isCurrent && 'ring-2 ring-primary/30',
                  isCurrent && 'opacity-60',
                )}
              >
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <CardTitle>{plan.name}</CardTitle>
                    <div className="flex items-center gap-2">
                      {isCurrent && (
                        <Badge variant="secondary" className="shrink-0">
                          Plan actuel
                        </Badge>
                      )}
                      {plan.featured && !isCurrent && (
                        <Badge variant="default" className="shrink-0">
                          <Zap className="size-3" data-icon="inline-start" />
                          Populaire
                        </Badge>
                      )}
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
                      <div key={feature} className="flex items-center gap-2">
                        <Check className="size-3.5 text-primary shrink-0" />
                        <span className="text-sm text-muted-foreground">{feature}</span>
                      </div>
                    ))}
                  </div>
                </CardContent>
                <CardFooter>
                  <Button
                    variant={plan.featured && !isCurrent ? 'default' : 'outline'}
                    className="w-full"
                    onClick={() => handleCheckout(plan.code)}
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
          })}
        </div>
      </div>
    </div>
  );
}
