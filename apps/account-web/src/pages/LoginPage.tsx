import { LoginBrandPanel } from './LoginBrandPanel';
import { LoginPageContent } from './LoginPage.content';
import { useLoginPage } from './useLoginPage';
import { useLoginPageTransition } from './useLoginPage.transition';

export default function LoginPage() {
  const page = useLoginPage();
  const transitionDirection = useLoginPageTransition(page.step);

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)]">
      {!page.checkingAuth && <LoginBrandPanel />}
      <LoginPageContent {...page} transitionDirection={transitionDirection} />
    </div>
  );
}
