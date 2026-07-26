import { StepUpModal, type StepUpModalProps } from './StepUpModal';

export { StepUpModal };
export type { StepUpModalProps };

interface StepUpFormProps {
  onSuccess: () => void;
  onCancel: () => void;
  description?: string;
}

export default function StepUpForm({ onSuccess, onCancel, description }: StepUpFormProps) {
  return (
    <StepUpModal open={true} onSuccess={onSuccess} onCancel={onCancel} description={description} />
  );
}
