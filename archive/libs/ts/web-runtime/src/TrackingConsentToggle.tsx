import * as Switch from '@radix-ui/react-switch';

export interface TrackingConsentToggleProps {
  checked: boolean;
  onChange: () => void;
  label: string;
  description: string;
  large?: boolean;
}

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
      <Switch.Root
        checked={checked}
        onCheckedChange={onChange}
        aria-label={label}
        className={
          large
            ? 'relative h-5 w-9 shrink-0 rounded-full bg-muted outline-none transition-colors data-[state=checked]:bg-primary focus-visible:ring-3 focus-visible:ring-ring/50'
            : 'relative h-4 w-8 shrink-0 rounded-full bg-muted outline-none transition-colors data-[state=checked]:bg-primary focus-visible:ring-3 focus-visible:ring-ring/50'
        }
      >
        <Switch.Thumb
          className={
            large
              ? 'block size-4 translate-x-0.5 rounded-full border border-border bg-background shadow-sm transition-transform data-[state=checked]:translate-x-[18px]'
              : 'block size-3 translate-x-0.5 rounded-full border border-border bg-background shadow-sm transition-transform data-[state=checked]:translate-x-[18px]'
          }
        />
      </Switch.Root>
    </div>
  );
}
