export const subscriptionStatusLabels: Record<string, string> = {
  active: 'Actif',
  trialing: 'Essai',
  past_due: 'En retard',
  canceled: 'Annule',
  incomplete: 'Incomplet',
};

export const subscriptionStatusVariants: Record<
  string,
  'default' | 'secondary' | 'destructive' | 'outline'
> = {
  active: 'default',
  trialing: 'secondary',
  past_due: 'destructive',
  canceled: 'outline',
  incomplete: 'outline',
};

export function formatSubscriptionCents(cents: number, currency: string): string {
  return new Intl.NumberFormat('fr-FR', {
    style: 'currency',
    currency: currency.toUpperCase(),
  }).format(cents / 100);
}

export function formatSubscriptionDate(dateStr: string | null | undefined): string {
  if (!dateStr) {
    return '—';
  }

  return new Date(dateStr).toLocaleDateString('fr-FR', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}
