import { CheckIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';

export function DeviceActivationSuccessStep({ onGoAccount }: { onGoAccount: () => void }) {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <CheckIcon />
        </EmptyMedia>
        <EmptyTitle>Appareil activé !</EmptyTitle>
        <EmptyDescription>
          Vous pouvez maintenant retourner sur votre appareil pour continuer.
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent className="w-full">
        <Button variant="outline" onClick={onGoAccount} className="w-full">
          Aller à mon compte
        </Button>
      </EmptyContent>
    </Empty>
  );
}
