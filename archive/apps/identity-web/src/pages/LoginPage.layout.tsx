import type { ReactNode } from 'react';
import { Spinner } from '@/components/ui/spinner';
import type { LoginStep } from './LoginProgress';

export function LoginPageLoading() {
  return (
    <div className="flex flex-1 items-center justify-center bg-muted/30">
      <Spinner className="size-6 text-primary" />
    </div>
  );
}

export function LoginPageCard({
  step,
  transitionDirection = 'forward',
  footer,
  children,
}: {
  step: LoginStep;
  transitionDirection?: 'forward' | 'backward';
  footer?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div
      data-login-step={step}
      className={
        transitionDirection === 'backward'
          ? 'animate-login-card-enter-backward motion-reduce:animate-none'
          : 'animate-login-card-enter-forward motion-reduce:animate-none'
      }
    >
      {children}
      {footer ? (
        <div
          className={
            step === 'identifier'
              ? '-mt-11 flex h-11 items-center justify-start'
              : 'mt-7 flex justify-start'
          }
        >
          {footer}
        </div>
      ) : null}
    </div>
  );
}
