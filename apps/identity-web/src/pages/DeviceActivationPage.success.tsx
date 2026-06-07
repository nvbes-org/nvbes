import { Button } from '@/components/ui/button';

export function DeviceActivationSuccessStep({ onGoAccount }: { onGoAccount: () => void }) {
  return (
    <div className="space-y-6 py-4 text-center">
      <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-green-100">
        <svg
          className="h-8 w-8 text-green-600"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          role="img"
          aria-label="Succès"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
        </svg>
      </div>
      <div className="space-y-2">
        <h2 className="text-xl font-bold">Appareil activé !</h2>
        <p className="text-sm text-muted-foreground">
          Vous pouvez maintenant retourner sur votre appareil pour continuer.
        </p>
      </div>
      <Button variant="outline" onClick={onGoAccount} className="w-full">
        Aller à mon compte
      </Button>
    </div>
  );
}
