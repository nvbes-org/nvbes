import { useNavigate } from '@tanstack/react-router';
import { Button } from '@/components/ui/button';

export default function NotFoundPage() {
  const navigate = useNavigate();

  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="text-center space-y-6">
        <h1 className="text-6xl font-bold text-muted-foreground">404</h1>
        <p className="text-xl text-muted-foreground">Page introuvable</p>
        <Button onClick={() => void navigate({ to: '/login' })}>Retour à la connexion</Button>
      </div>
    </div>
  );
}
