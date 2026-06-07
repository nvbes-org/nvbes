import { ArrowLeft } from 'lucide-react';
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
  tokenFromUrl,
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
      <button
        type="button"
        className="mb-6 flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
        onClick={navigateToLogin}
      >
        <ArrowLeft className="size-3.5" />
        Retour a la connexion
      </button>

      <Card>
        <CardHeader>
          <CardTitle>Nouveau mot de passe</CardTitle>
          <CardDescription>Choisissez un nouveau mot de passe pour votre compte.</CardDescription>
        </CardHeader>
        <CardContent>
          {success ? (
            <ResetPasswordSuccessState navigateToLogin={navigateToLogin} />
          ) : (
            <ResetPasswordForm
              tokenFromUrl={tokenFromUrl}
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
