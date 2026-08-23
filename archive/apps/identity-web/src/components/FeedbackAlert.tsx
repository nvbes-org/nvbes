import type { ReactNode } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';

export function FeedbackAlert({
  children,
  tone = 'success',
}: {
  children: ReactNode;
  tone?: 'error' | 'success';
}) {
  return (
    <Alert variant={tone === 'error' ? 'destructive' : 'default'}>
      <AlertDescription>{children}</AlertDescription>
    </Alert>
  );
}
