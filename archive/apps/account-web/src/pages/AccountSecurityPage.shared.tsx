import type { ReactNode } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';

export function SecuritySection({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="space-y-4 border-t border-border pt-6">
      <div>
        <h2 className="text-lg font-semibold">{title}</h2>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      {children}
    </section>
  );
}

export function MutationStatus({
  mutation,
  success,
}: {
  mutation: { error: Error | null; isSuccess: boolean };
  success?: string;
}) {
  if (mutation.error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{mutation.error.message}</AlertDescription>
      </Alert>
    );
  }
  if (mutation.isSuccess && success) {
    return (
      <Alert>
        <AlertDescription>{success}</AlertDescription>
      </Alert>
    );
  }
  return null;
}
