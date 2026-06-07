import { FileSearch } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function GdprExportsCard() {
  return (
    <Card className="animate-fade-slide-up [animation-delay:100ms]">
      <CardHeader>
        <CardTitle>Exports RGPD</CardTitle>
        <CardDescription>Conformite RGPD et demandes d&apos;acces aux donnees.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <Button
          variant="outline"
          className="w-full justify-between"
          onClick={() => window.open('/account/privacy', '_self')}
        >
          Demander l&apos;export de mes donnees
          <FileSearch className="size-4" data-icon="inline-end" />
        </Button>
        <p className="text-xs text-muted-foreground">
          Conformement au RGPD, vous pouvez demander une copie de toutes vos donnees personnelles ou
          la suppression complete de votre compte.
        </p>
      </CardContent>
    </Card>
  );
}
