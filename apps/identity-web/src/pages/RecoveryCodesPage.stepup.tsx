import StepUpForm from '@/components/StepUpForm';

export function RecoveryCodesStepUp({
  onCancel,
  onSuccess,
}: {
  onCancel: () => void;
  onSuccess: () => void;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center">
      <StepUpForm
        onSuccess={onSuccess}
        onCancel={onCancel}
        description="Pour générer des codes de récupération, veuillez confirmer votre identité."
      />
    </div>
  );
}
