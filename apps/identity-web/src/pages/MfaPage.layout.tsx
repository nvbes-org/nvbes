import { ShieldAlert } from 'lucide-react';
import { IdentityEmptyState } from '@/components/IdentityEmptyState';
import { IdentityPageHeader } from '@/components/IdentityPage';
import { Card, CardHeader } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';

export function MfaPageSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <Skeleton className="h-6 w-56" />
        <Skeleton className="h-4 w-72" />
      </div>
      {Array.from({ length: 2 }).map((_, index) => (
        <Card key={index}>
          <CardHeader>
            <div className="flex items-center gap-3">
              <Skeleton className="size-9 shrink-0 rounded-lg" />
              <div className="flex flex-col gap-1.5">
                <Skeleton className="h-4 w-36" />
                <Skeleton className="h-3 w-24" />
              </div>
            </div>
          </CardHeader>
        </Card>
      ))}
    </div>
  );
}

export function MfaPageHeader({ onBack }: { onBack: () => void }) {
  return (
    <IdentityPageHeader
      title="Authentification multi-facteurs"
      description="Gérez les méthodes de vérification de votre compte."
      onBack={onBack}
    />
  );
}

export function MfaPageEmptyState() {
  return (
    <IdentityEmptyState
      icon={ShieldAlert}
      title="Aucun facteur configuré."
      description={
        <>
          Ajoutez une méthode d&apos;authentification
          <br />
          ci-dessous pour sécuriser votre compte.
        </>
      }
    />
  );
}
