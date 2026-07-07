import { ResetPasswordPageCard } from './ResetPasswordPage.shared';
import { useResetPasswordPage } from './useResetPasswordPage';

export default function ResetPasswordPage() {
  const page = useResetPasswordPage();

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/30 p-4 sm:p-8">
      <ResetPasswordPageCard {...page} />
    </div>
  );
}
