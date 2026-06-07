import { ShieldCheckIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';

const SCOPE_DESCRIPTIONS: Record<string, { label: string; desc: string }> = {
  openid: { label: 'Authentification', desc: 'Confirmer votre identité' },
  profile: { label: 'Profil utilisateur', desc: 'Accéder à votre nom et photo de profil' },
  email: { label: 'Adresse email', desc: 'Voir votre adresse email principale' },
  offline_access: {
    label: 'Accès hors connexion',
    desc: 'Maintenir la connexion sans vous reconnecter',
  },
  'drive:read': { label: 'Lecture Drive', desc: 'Consulter et lire vos fichiers et dossiers' },
  'drive:write': {
    label: 'Écriture Drive',
    desc: 'Créer, modifier et organiser vos fichiers et dossiers',
  },
  'drive:admin': {
    label: 'Administration Drive',
    desc: 'Gérer tous les paramètres de votre espace de stockage',
  },
};

export function LoginPageConsent({
  scope,
  error,
  onApprove,
  onCancel,
}: {
  scope: string | null | undefined;
  error: string | null;
  onApprove: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="flex flex-col gap-5">
      <div className="rounded-2xl border bg-muted/20 p-4">
        <p className="mb-3 text-xs font-semibold uppercase tracking-wider text-foreground/80">
          Autorisations demandées :
        </p>
        <div className="flex flex-col gap-3.5">
          {(scope?.split(/\s+/) ?? []).map((requestedScope) => {
            const description = SCOPE_DESCRIPTIONS[requestedScope] || {
              label: requestedScope,
              desc: "Scope requis par l'application",
            };
            return (
              <div key={requestedScope} className="flex items-start gap-2.5">
                <div className="mt-0.5 rounded-full bg-primary/10 p-1 text-primary">
                  <ShieldCheckIcon className="size-3.5" />
                </div>
                <div>
                  <p className="text-xs font-semibold text-foreground/85">{description.label}</p>
                  <p className="text-[11px] text-muted-foreground">{description.desc}</p>
                </div>
              </div>
            );
          })}
        </div>
      </div>
      {error && (
        <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      )}
      <div className="flex gap-2">
        <Button type="button" variant="outline" className="flex-1" onClick={onCancel}>
          Refuser
        </Button>
        <Button type="button" className="flex-1" onClick={onApprove}>
          Autoriser
        </Button>
      </div>
    </div>
  );
}
