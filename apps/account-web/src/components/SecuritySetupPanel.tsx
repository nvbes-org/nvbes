import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export function SecuritySetupPanel({
  title,
  children,
  className,
  titleClassName,
}: {
  title: ReactNode;
  children: ReactNode;
  className?: string;
  titleClassName?: string;
}) {
  return (
    <div className={cn('mx-auto w-full max-w-md space-y-4', className)}>
      <h1 className={cn('text-2xl font-bold', titleClassName)}>{title}</h1>
      {children}
    </div>
  );
}

export function SecuritySetupActions({ children }: { children: ReactNode }) {
  return <div className="flex gap-2">{children}</div>;
}

export function SecuritySetupSuccess({
  title,
  description,
  onBack,
}: {
  title: string;
  description: ReactNode;
  onBack: () => void;
}) {
  return (
    <SecuritySetupPanel title={title} className="space-y-6 text-center">
      <p className="text-muted-foreground">{description}</p>
      <Button onClick={onBack}>Retour à la sécurité</Button>
    </SecuritySetupPanel>
  );
}
