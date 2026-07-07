import { KeyRound } from 'lucide-react';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';

export function EmptyClientState() {
  return (
    <Empty className="border">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <KeyRound />
        </EmptyMedia>
        <EmptyTitle>Aucun client OAuth</EmptyTitle>
        <EmptyDescription>
          Creer ou attacher un client pour emettre des tokens M2M.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
