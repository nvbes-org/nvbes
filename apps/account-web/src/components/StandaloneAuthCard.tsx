import { ArrowLeft } from 'lucide-react';
import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';

export function StandaloneAuthCard({
  title,
  description,
  children,
  footer,
  onBack,
  backLabel = 'Retour a la connexion',
}: {
  title: ReactNode;
  description: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  onBack: () => void;
  backLabel?: string;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/30 p-4 sm:p-8">
      <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
        <Button
          type="button"
          variant="ghost"
          className="mb-6 px-0 text-muted-foreground hover:text-foreground"
          onClick={onBack}
        >
          <ArrowLeft data-icon="inline-start" />
          {backLabel}
        </Button>
        <Card>
          <CardHeader>
            <CardTitle role="heading" aria-level={1}>
              {title}
            </CardTitle>
            <CardDescription>{description}</CardDescription>
          </CardHeader>
          <CardContent>{children}</CardContent>
          {footer ? <CardFooter className="justify-center">{footer}</CardFooter> : null}
        </Card>
      </div>
    </div>
  );
}
