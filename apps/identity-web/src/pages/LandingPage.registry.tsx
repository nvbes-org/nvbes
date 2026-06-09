import { X } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';

export function ShadcnblocksAnnouncement() {
  const [visible, setVisible] = useState(true);

  if (!visible) return null;

  return (
    <section className="w-full bg-muted px-4 py-3">
      <div className="mx-auto flex max-w-7xl items-center gap-2">
        <p className="flex-1 text-sm">
          <span className="font-medium">nvbes V0 est disponible.</span>{' '}
          <span className="text-muted-foreground">
            Identity et Drive partagent maintenant le meme systeme de composants.
          </span>
        </p>
        <Button
          variant="ghost"
          size="icon"
          className="-mr-2 size-8 flex-none"
          onClick={() => setVisible(false)}
          aria-label="Masquer l'annonce"
        >
          <X />
        </Button>
      </div>
    </section>
  );
}

export function TailarkStatsSection() {
  return (
    <section className="bg-muted py-12 md:py-16">
      <div className="mx-auto max-w-5xl px-6">
        <Card className="grid gap-0.5 divide-y p-0 text-center md:grid-cols-3 md:divide-x md:divide-y-0">
          <CardContent className="p-8">
            <div className="text-4xl font-bold text-foreground">3</div>
            <p className="mt-2 text-sm text-muted-foreground">Produits connectes</p>
          </CardContent>
          <CardContent className="p-8">
            <div className="text-4xl font-bold text-foreground">100%</div>
            <p className="mt-2 text-sm text-muted-foreground">Workflows MFA couverts</p>
          </CardContent>
          <CardContent className="p-8">
            <div className="text-4xl font-bold text-foreground">0</div>
            <p className="mt-2 text-sm text-muted-foreground">Acces direct base requis</p>
          </CardContent>
        </Card>
      </div>
    </section>
  );
}

export function TailwindadminProgressPanel() {
  const rows = [
    { label: 'Identity', value: 92 },
    { label: 'Drive', value: 78 },
    { label: 'Audit', value: 64 },
  ];

  return (
    <section className="border-b border-border bg-background py-12">
      <div className="mx-auto grid max-w-5xl gap-6 px-6 md:grid-cols-[0.9fr_1.1fr] md:items-center">
        <div>
          <h2 className="text-2xl font-semibold tracking-normal">Pilotage admin</h2>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            Une lecture rapide des domaines sensibles, inspiree des dashboards Tailwindadmin.
          </p>
        </div>
        <Card>
          <CardContent className="grid gap-4 p-5">
            {rows.map((row) => (
              <div key={row.label} className="grid gap-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="font-medium">{row.label}</span>
                  <span className="text-muted-foreground">{row.value}%</span>
                </div>
                <Progress value={row.value} />
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    </section>
  );
}
