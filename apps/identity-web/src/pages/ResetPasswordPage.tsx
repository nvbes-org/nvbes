import { ResetPasswordPageCard } from './ResetPasswordPage.shared';
import { useResetPasswordPage } from './useResetPasswordPage';

export default function ResetPasswordPage() {
  const page = useResetPasswordPage();
  return <ResetPasswordPageCard {...page} />;
}
