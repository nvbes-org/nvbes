import type { TrackingConsentToggleProps } from '@nvbes/web-runtime';
import { Switch } from '@/components/ui/switch';

export function TrackingConsentToggle({
  checked,
  onChange,
  label,
  description,
  large = false,
}: TrackingConsentToggleProps) {
  return (
    <div
      className={
        large
          ? 'flex items-center justify-between gap-4 rounded-xl border border-border/50 bg-background/70 p-3'
          : 'flex items-center justify-between gap-3'
      }
    >
      <div className="min-w-0">
        <p
          className={
            large
              ? 'text-xs font-medium text-foreground'
              : 'text-[11px] font-medium text-foreground'
          }
        >
          {label}
        </p>
        <p
          className={
            large ? 'text-[11px] text-muted-foreground' : 'text-[10px] text-muted-foreground'
          }
        >
          {description}
        </p>
      </div>
      <Switch
        checked={checked}
        onCheckedChange={onChange}
        aria-label={label}
        size={large ? 'default' : 'sm'}
        className="shrink-0"
      />
    </div>
  );
}
