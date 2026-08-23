import { Button } from '@/components/ui/button';

export function ResultPrimaryAction({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <Button className="w-full" onClick={onClick}>
      {label}
    </Button>
  );
}

export function ResultSecondaryAction({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <Button variant="outline" className="w-full" onClick={onClick}>
      {label}
    </Button>
  );
}
