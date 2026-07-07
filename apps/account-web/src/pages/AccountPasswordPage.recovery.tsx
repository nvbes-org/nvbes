import { Key } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function AccountPasswordRecoveryCard() {
  return (
    <Card className="animate-fade-slide-up [animation-delay:100ms]">
      <CardHeader>
        <CardTitle>Mot de passe oublie</CardTitle>
        <CardDescription>
          Si vous ne vous souvenez plus de votre mot de passe actuel, utilisez le lien de
          recuperation.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Button variant="outline" className="w-full justify-between" asChild>
          <a href="/forgot-password">
            Reinitialiser via email
            <Key className="size-4" data-icon="inline-end" />
          </a>
        </Button>
      </CardContent>
    </Card>
  );
}
