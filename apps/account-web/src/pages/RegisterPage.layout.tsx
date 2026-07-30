import { Card, CardContent, CardFooter } from '@/components/ui/card';
import type { ReactNode } from 'react';
import { LoginPageLegalLinks } from './LoginPage.forms';

export function RegisterPageShell({
  title,
  searchStr,
  children,
}: {
  title: string;
  searchStr: string;
  children: ReactNode;
}) {
  return (
    <div className="flex w-full flex-1 items-center justify-center bg-muted/30 p-4 sm:p-8">
      <div className="w-full max-w-md animate-fade-slide-up [animation-delay:150ms]">
        <header className="mb-8">
          <h1 className="font-heading text-3xl font-semibold">{title}</h1>
        </header>

        <Card>
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
        <LoginPageLegalLinks />
      </div>
    </div>
  );
}
