import { Alert, AlertDescription } from '@/components/ui/alert';

export function AccountPasswordErrorMessage({ message }: { message: string }) {
  return (
    <Alert variant="destructive">
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}

export function AccountPasswordSuccessMessage({ message }: { message: string }) {
  return (
    <Alert>
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}
