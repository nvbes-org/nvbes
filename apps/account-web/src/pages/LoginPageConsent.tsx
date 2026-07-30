import { sanitizeStyleElementCss, sanitizeUrlForAttribute } from '@nvbes/web-runtime';
import { ShieldCheckIcon } from 'lucide-react';
import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';

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
  clientName,
  scope,
  error,
  logoUrl,
  description: customDescription,
  supportUrl,
  privacyUrl,
  termsUrl,
  brandColor,
  customCss,
  helpText,
  onApprove,
  onCancel,
}: {
  clientName?: string;
  scope: string | null | undefined;
  error: string | null;
  logoUrl?: string | null;
  description?: string | null;
  supportUrl?: string | null;
  privacyUrl?: string | null;
  termsUrl?: string | null;
  brandColor?: string | null;
  customCss?: string | null;
  helpText?: string | null;
  onApprove: () => void;
  onCancel: () => void;
}) {
  const styleOverrides = brandColor
    ? ({
        '--primary': brandColor,
        '--ring': brandColor,
      } as React.CSSProperties)
    : {};
  const safeCustomCss = sanitizeStyleElementCss(customCss);
  const safeLogoUrl = sanitizeUrlForAttribute(logoUrl, {
    allowRelative: false,
    allowedProtocols: ['https:'],
  });
  const safePrivacyUrl = sanitizeUrlForAttribute(privacyUrl, {
    allowRelative: false,
    allowedProtocols: ['https:'],
  });
  const safeTermsUrl = sanitizeUrlForAttribute(termsUrl, {
    allowRelative: false,
    allowedProtocols: ['https:'],
  });
  const safeSupportUrl = sanitizeUrlForAttribute(supportUrl, {
    allowRelative: false,
    allowedProtocols: ['https:'],
  });

  return (
    <div className="flex flex-col gap-5" style={styleOverrides}>
      {safeCustomCss ? <style>{safeCustomCss}</style> : null}
      <Card className="p-5">
        <div className="mb-4 flex items-center gap-3">
          {safeLogoUrl ? (
            <img
              src={safeLogoUrl}
              alt={clientName || 'Application Logo'}
              className="size-10 rounded-md border border-border bg-background object-contain p-1"
            />
          ) : (
            <div className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
              <ShieldCheckIcon className="size-6" />
            </div>
          )}
          <div>
            <h3 className="text-sm font-semibold text-foreground">{clientName || 'Application'}</h3>
            <p className="text-[11px] text-muted-foreground">Demande d'autorisation d'accès</p>
          </div>
        </div>

        <p className="mb-3 text-xs text-muted-foreground/90">
          {customDescription ||
            'Cette application souhaite se connecter à votre compte nvbes pour accéder aux autorisations suivantes.'}
        </p>

        <p className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/70">
          Autorisations requises :
        </p>
        <div className="flex flex-col gap-3">
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

        {helpText && (
          <div className="mt-4 border-t border-border/50 pt-3 text-[11px] text-muted-foreground whitespace-pre-line leading-relaxed">
            {helpText}
          </div>
        )}
      </Card>
      {error && <FeedbackAlert tone="error">{error}</FeedbackAlert>}
      <div className="flex flex-col gap-2.5">
        <div className="flex gap-2">
          <Button type="button" variant="outline" className="flex-1" onClick={onCancel}>
            Refuser
          </Button>
          <Button type="button" className="flex-1" onClick={onApprove}>
            Autoriser
          </Button>
        </div>
        {(safePrivacyUrl || safeTermsUrl || safeSupportUrl) && (
          <div className="flex justify-center gap-3 text-[10px] text-muted-foreground">
            {safePrivacyUrl && (
              <a
                href={safePrivacyUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-foreground hover:underline transition-colors"
              >
                Politique de confidentialité
              </a>
            )}
            {safePrivacyUrl && (safeTermsUrl || safeSupportUrl) && (
              <span className="text-muted-foreground/30">•</span>
            )}
            {safeTermsUrl && (
              <a
                href={safeTermsUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-foreground hover:underline transition-colors"
              >
                Conditions d'utilisation
              </a>
            )}
            {safeTermsUrl && safeSupportUrl && <span className="text-muted-foreground/30">•</span>}
            {safeSupportUrl && (
              <a
                href={safeSupportUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-foreground hover:underline transition-colors"
              >
                Support
              </a>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
