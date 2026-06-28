import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import type { ReactNode } from 'react';
import { LoginPageLegalLinks } from './LoginPage.forms';

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
        <LoginPageLegalLinks />
      </div>
    </div>
  );
}
