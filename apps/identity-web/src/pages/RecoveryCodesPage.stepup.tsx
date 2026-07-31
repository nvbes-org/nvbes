import StepUpModal from '@/components/StepUpModal';

export function RecoveryCodesStepUp({
  open = true,
  onCancel,
  onSuccess,
}: {
  open?: boolean;
  onCancel: () => void;
  onSuccess: () => void;
}) {
  return (
    <StepUpModal
      open={open}
      onSuccess={onSuccess}
      onCancel={onCancel}
      description="Pour générer des codes de récupération, veuillez confirmer votre identité."
    />
  );
}
