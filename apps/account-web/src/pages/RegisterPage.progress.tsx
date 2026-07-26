import { CheckCircle2Icon, MapPinIcon, type LucideIcon } from 'lucide-react';

import { cn } from '@/lib/classnames';

export type RegisterStep = 1 | 2;

const STEPS: { label: string; icon: LucideIcon }[] = [
  { label: 'Compte', icon: CheckCircle2Icon },
  { label: 'Région', icon: MapPinIcon },
];

export function RegisterProgress({ step }: { step: RegisterStep }) {
  return (
    <div className="mb-8">
      <div className="flex items-center gap-2">
        {STEPS.map((entry, index) => {
          const stepNum = index + 1;
          const active = step === stepNum;
          const complete = step > stepNum;
          const Icon = entry.icon;

          return (
            <div key={entry.label} className="flex items-center gap-2">
              <div
                className={cn(
                  'flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-medium transition-all duration-500',
                  active && 'bg-primary text-primary-foreground shadow-sm',
                  complete && 'bg-primary/10 text-primary',
                  !active && !complete && 'bg-muted text-muted-foreground',
                )}
              >
                <Icon className="size-3.5" />
                <span>
                  {stepNum}. {entry.label}
                </span>
              </div>
              {index < STEPS.length - 1 && (
                <div
                  className={cn(
                    'h-px w-8 transition-colors duration-500',
                    complete ? 'bg-primary/50' : 'bg-border',
                  )}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
