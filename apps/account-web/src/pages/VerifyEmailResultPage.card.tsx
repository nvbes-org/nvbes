import type { ReactNode } from 'react';
import { Card, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';

export function ResultCard({
  title,
  description,
  icon,
  actions,
}: {
  title: string;
  description: ReactNode;
  icon: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <Card className="w-full max-w-sm">
      <CardHeader className="pb-2 text-center">
        <div className="mx-auto mb-3">{icon}</div>
        <CardTitle>{title}</CardTitle>
        <CardDescription className="leading-relaxed">{description}</CardDescription>
      </CardHeader>
      {actions ? <CardFooter className="flex-col gap-3 pt-2">{actions}</CardFooter> : null}
    </Card>
  );
}
