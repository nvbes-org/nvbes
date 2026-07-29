import { ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { ResetPasswordForm } from './ResetPasswordPage.form';
import { ResetPasswordSuccessState } from './ResetPasswordPage.success';
import type { ResetPasswordPageModel } from './ResetPasswordPage.types';

export function ResetPasswordPageCard({
  tokenFromLink,
  token,
  password,
  confirmPassword,
  error,
  success,
  isPending,
  navigateToLogin,
  setToken,
  setPassword,
  setConfirmPassword,
  handleSubmit,
}: ResetPasswordPageModel) {
  return (
    <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
      <Button
        type="button"
        variant="ghost"
        className="mb-6 px-0 text-muted-foreground hover:text-foreground"
        onClick={navigateToLogin}
      >
        <ArrowLeft data-icon="inline-start" />
        Retour a la connexion
      </Button>

      <Card>
        <CardHeader>
          <CardTitle role="heading" aria-level={1}>
            Nouveau mot de passe
          </CardTitle>
          <CardDescription>Choisissez un nouveau mot de passe pour votre compte.</CardDescription>
        </CardHeader>
        <CardContent>
          {success ? (
            <ResetPasswordSuccessState navigateToLogin={navigateToLogin} />
          ) : (
            <ResetPasswordForm
              tokenFromLink={tokenFromLink}
              token={token}
              password={password}
              confirmPassword={confirmPassword}
              error={error}
              isPending={isPending}
              setToken={setToken}
              setPassword={setPassword}
              setConfirmPassword={setConfirmPassword}
              handleSubmit={handleSubmit}
            />
          )}
        </CardContent>
        <CardFooter className="justify-center">
          <p className="text-center text-sm text-muted-foreground">
            <a href="/forgot-password" className="font-medium text-primary hover:underline">
              Renvoyer le lien
            </a>
          </p>
        </CardFooter>
      </Card>
    </div>
  );
}
