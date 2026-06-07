import type { RecoveryReviewView } from '@/pages/RecoveryReviewsPage.api';

export function formatRecoveryReviewDateTime(value: string | null): string {
  return value ? new Date(value).toLocaleString() : '—';
}

export function recoveryReviewStatus(review: RecoveryReviewView): string {
  if (review.secondary_approved_at) {
    return 'Double approbation obtenue';
  }

  if (review.review_available_at) {
    return `Revue disponible à ${formatRecoveryReviewDateTime(review.review_available_at)}`;
  }

  return 'En attente de première approbation';
}
