export function TrackingConsentToggle({
  checked,
  onChange,
  label,
  description,
  large = false,
}: {
  checked: boolean;
  onChange: () => void;
  label: string;
  description: string;
  large?: boolean;
}) {
  const sizeClass = large ? 'w-9 h-5 after:h-4 after:w-4' : 'w-8 h-4 after:h-3 after:w-3';

  return (
    <div className="flex items-center justify-between gap-3">
      <div>
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
      <label className="relative inline-flex shrink-0 cursor-pointer items-center">
        <input type="checkbox" checked={checked} onChange={onChange} className="sr-only peer" />
        <div
          className={`${sizeClass} rounded-full bg-muted peer peer-checked:bg-primary peer-focus:outline-none peer-checked:after:translate-x-full peer-checked:after:border-white after:absolute after:left-[2px] after:top-[2px] after:h-3 after:w-3 after:rounded-full after:border after:border-border after:bg-background after:transition-all after:content-['']`}
        />
      </label>
    </div>
  );
}
