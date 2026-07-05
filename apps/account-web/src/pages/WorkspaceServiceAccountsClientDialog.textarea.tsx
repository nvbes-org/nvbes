import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';

export function ClientTextarea({
  id,
  label,
  value,
  onChange,
  className,
  placeholder,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  className?: string;
  placeholder?: string;
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Textarea
        id={id}
        className={className ?? 'min-h-20'}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
      />
    </div>
  );
}
