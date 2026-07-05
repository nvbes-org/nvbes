import { CircleAlertIcon, ClockIcon, ShieldAlertIcon } from 'lucide-react';
import { ResultPrimaryAction, ResultSecondaryAction } from './VerifyEmailResultPage.actions';
import { ResultCard } from './VerifyEmailResultPage.card';
import { ResultStatusIcon } from './VerifyEmailResultPage.status_icon';

export function VerifyEmailExpiredState({
  onGoLogin,
  onGoVerify,
}: {
  onGoLogin: () => void;
  onGoVerify: () => void;
}) {
  return (
    <ResultCard
      title="Lien expiré"
      description="Ce lien de vérification a expiré. Demande un nouveau lien pour vérifier ton adresse email."
      icon={
        <ResultStatusIcon tone="amber">
          <ClockIcon className="size-7 text-amber-600" />
        </ResultStatusIcon>
      }
      actions={
        <>
          <ResultPrimaryAction label="Renvoyer le lien" onClick={onGoVerify} />
          <ResultSecondaryAction label="Retour à la connexion" onClick={onGoLogin} />
        </>
      }
    />
  );
}

export function VerifyEmailInvalidState({
  onGoLogin,
  onGoVerify,
}: {
  onGoLogin: () => void;
  onGoVerify: () => void;
}) {
  return (
    <ResultCard
      title="Lien invalide"
      description="Ce lien de vérification n'est pas valide. Il a peut-être déjà été utilisé ou est incorrect."
      icon={
        <ResultStatusIcon tone="red">
          <CircleAlertIcon className="size-7 text-red-500" />
        </ResultStatusIcon>
      }
      actions={
        <>
          <ResultPrimaryAction label="Demander un nouveau lien" onClick={onGoVerify} />
          <ResultSecondaryAction label="Retour à la connexion" onClick={onGoLogin} />
        </>
      }
    />
  );
}

export function VerifyEmailErrorState({
  message,
  onGoLogin,
  onGoVerify,
}: {
  message: string;
  onGoLogin: () => void;
  onGoVerify: () => void;
}) {
  return (
    <ResultCard
      title="Erreur"
      description={message}
      icon={
        <ResultStatusIcon tone="red">
          <ShieldAlertIcon className="size-7 text-red-500" />
        </ResultStatusIcon>
      }
      actions={
        <>
          <ResultPrimaryAction label="Réessayer" onClick={onGoVerify} />
          <ResultSecondaryAction label="Retour à la connexion" onClick={onGoLogin} />
        </>
      }
    />
  );
}
