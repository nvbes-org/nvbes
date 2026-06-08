import { UserCircle2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { DriveMeResponse } from './drive.api';

export function DriveAccountView({ me }: { me: DriveMeResponse }) {
  return (
    <section className="grid gap-4">
      <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
        <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
          <div className="flex min-w-0 items-start gap-3">
            <span className="grid size-12 shrink-0 place-items-center rounded-2xl bg-primary/10 text-primary">
              <UserCircle2 className="size-6" aria-hidden="true" />
            </span>
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                Compte Identity
              </p>
              <h2 className="mt-1 truncate text-lg font-semibold">{me.user.display_name}</h2>
              <p className="truncate text-sm text-muted-foreground">{me.user.email}</p>
            </div>
          </div>
          <Button type="button" variant="outline" disabled title="URL Identity non configuree dans Drive.">
            Ouvrir Identity indisponible
          </Button>
        </div>
      </article>

      <article className="rounded-2xl border border-border/70 bg-background p-4 shadow-sm">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
          Handoff
        </p>
        <h3 className="mt-1 font-medium">Continuer dans Identity</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          Utilisez le compte {me.user.email} pour gerer le profil, les sessions et les facteurs
          d'authentification depuis Identity. Aucun lien externe n'est affiche tant que l'URL
          Identity n'est pas fournie par la configuration Drive.
        </p>
      </article>
    </section>
  );
}
