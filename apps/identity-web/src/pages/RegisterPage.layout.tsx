import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import type { ReactNode } from 'react';

export function RegisterPageShell({
  title,
  description,
  searchStr,
  children,
}: {
  title: string;
  description: string;
  searchStr: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-1 items-center justify-center bg-muted/30 p-4 sm:p-8">
      <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
        <div className="mb-8 flex items-center gap-2.5 lg:hidden">
          <div className="inline-flex size-8 items-center justify-center rounded-lg bg-primary/15">
            <div className="size-3 rounded-sm bg-primary" />
          </div>
          <span className="text-lg font-semibold tracking-tight text-foreground/85">nvbes</span>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>{title}</CardTitle>
            <CardDescription>{description}</CardDescription>
          </CardHeader>

          <CardContent>{children}</CardContent>

          <CardFooter className="justify-center">
            <p className="text-center text-sm text-muted-foreground">
              Déjà un compte ?{' '}
              <a href={`/login${searchStr}`} className="font-medium text-primary hover:underline">
                Se connecter
              </a>
            </p>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}
