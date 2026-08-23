import { useEffect, useRef } from 'react';
import type { LoginStep } from './LoginProgress';

const LOGIN_STEP_ORDER: Record<LoginStep, number> = {
  chooser: 0,
  identifier: 1,
  password: 2,
  webauthn: 2,
  mfa: 3,
  consent: 4,
};

export function useLoginPageTransition(step: LoginStep) {
  const previousStepRef = useRef<LoginStep | null>(null);
  const transitionDirection =
    previousStepRef.current !== null &&
    LOGIN_STEP_ORDER[step] < LOGIN_STEP_ORDER[previousStepRef.current]
      ? 'backward'
      : 'forward';

  useEffect(() => {
    previousStepRef.current = step;
  }, [step]);

  return transitionDirection;
}
