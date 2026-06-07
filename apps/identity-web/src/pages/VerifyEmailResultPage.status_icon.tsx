import type { ReactNode } from 'react';
import { AnimatedResultIcon } from './VerifyEmailResultPage.icon';

export function ResultStatusIcon({
  tone,
  children,
}: {
  tone: 'emerald' | 'amber' | 'red';
  children: ReactNode;
}) {
  const toneClasses = {
    emerald: 'bg-emerald-500/10 ring-emerald-500/20',
    amber: 'bg-amber-500/10 ring-amber-500/20',
    red: 'bg-red-500/10 ring-red-500/20',
  } as const;

  return (
    <AnimatedResultIcon>
      <div
        className={`flex size-14 items-center justify-center rounded-full ring-1 ${toneClasses[tone]}`}
      >
        {children}
      </div>
    </AnimatedResultIcon>
  );
}
