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
          ? 'flex items-center justify-between gap-4 px-4 py-3.5'
          : 'flex items-center justify-between gap-3'
      }
    >
      <div className="min-w-0">
        <p
          className={
            large
              ? 'text-sm font-semibold text-foreground'
              : 'text-[11px] font-medium text-foreground'
          }
        >
          {label}
        </p>
        <p
          className={
            large
              ? 'mt-0.5 text-xs leading-relaxed text-muted-foreground'
              : 'text-[10px] text-muted-foreground'
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
