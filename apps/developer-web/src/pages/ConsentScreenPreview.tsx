import { ShieldCheck } from 'lucide-react';

interface ConsentScreenPreviewProps {
  productName: string;
  description: string;
  logoUrl: string;
  supportUrl: string;
  privacyUrl: string;
  termsUrl: string;
  brandColor: string;
  customCss: string;
  helpText: string;
}

export function ConsentScreenPreview({
  productName,
  description,
  logoUrl,
  supportUrl,
  privacyUrl,
  termsUrl,
  brandColor,
  customCss,
  helpText,
}: ConsentScreenPreviewProps) {
  // Scope user custom CSS to only affect our preview wrapper
  const scopedCss = customCss
    ? customCss.replace(/([^\r\n,{}]+)(?=[^{}]*\{)/g, (match) => {
        return match
          .split(',')
          .map((selector) => `.consent-preview-wrapper ${selector.trim()}`)
          .join(', ');
      })
    : '';

  const styleOverrides = brandColor
    ? ({
        '--primary': brandColor,
        '--ring': brandColor,
      } as React.CSSProperties)
    : {};

  const dummyScopes = [
    { label: 'Authentification', desc: 'Confirmer votre identité (openid)' },
    { label: 'Profil utilisateur', desc: 'Accéder à votre nom et photo de profil (profile)' },
    { label: 'Adresse email', desc: 'Voir votre adresse email principale (email)' },
  ];

  return (
    <div className="consent-preview-wrapper h-full w-full rounded-lg border border-border bg-slate-50/50 p-6 dark:bg-slate-950/20" style={styleOverrides}>
      {scopedCss && <style dangerouslySetInnerHTML={{ __html: scopedCss }} />}
      
      <div className="mx-auto max-w-sm rounded-xl border border-border bg-card p-5 shadow-sm">
        {/* Header */}
        <div className="mb-4 flex items-center gap-3">
          {logoUrl ? (
            <img
              src={logoUrl}
              alt="Preview Logo"
              className="size-10 rounded-md border border-border bg-background object-contain p-1"
              onError={(e) => {
                // Fallback on broken URL
                e.currentTarget.style.display = 'none';
              }}
            />
          ) : (
            <div className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
              <ShieldCheck className="size-6" />
            </div>
          )}
          <div>
            <h3 className="text-sm font-semibold text-foreground">
              {productName || 'Nom du produit'}
            </h3>
            <p className="text-[10px] text-muted-foreground">Demande d'autorisation d'accès</p>
          </div>
        </div>

        {/* Description */}
        <p className="mb-3 text-xs text-muted-foreground/90">
          {description ||
            'Cette application souhaite se connecter à votre compte nvbes pour accéder aux autorisations suivantes.'}
        </p>

        {/* Scopes */}
        <p className="mb-2 text-[9px] font-semibold uppercase tracking-wider text-muted-foreground/70">
          Autorisations requises :
        </p>
        <div className="flex flex-col gap-2.5">
          {dummyScopes.map((scope) => (
            <div key={scope.label} className="flex items-start gap-2">
              <div className="mt-0.5 rounded-full bg-primary/10 p-0.5 text-primary">
                <ShieldCheck className="size-3" />
              </div>
              <div>
                <p className="text-[11px] font-semibold text-foreground/85">{scope.label}</p>
                <p className="text-[10px] text-muted-foreground">{scope.desc}</p>
              </div>
            </div>
          ))}
        </div>

        {/* Custom Help Text */}
        {helpText && (
          <div className="mt-4 border-t border-border/50 pt-3 text-[10px] text-muted-foreground whitespace-pre-line leading-relaxed">
            {helpText}
          </div>
        )}
      </div>

      {/* Footer links & buttons */}
      <div className="mx-auto mt-4 max-w-sm">
        <div className="flex gap-2">
          <button
            type="button"
            className="flex-1 rounded-md border border-input bg-background py-1.5 text-xs font-medium hover:bg-accent"
          >
            Refuser
          </button>
          <button
            type="button"
            className="flex-1 rounded-md bg-primary py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity"
          >
            Autoriser
          </button>
        </div>

        {/* Legal links */}
        {(privacyUrl || termsUrl || supportUrl) && (
          <div className="mt-3 flex flex-wrap justify-center gap-2.5 text-[9px] text-muted-foreground">
            {privacyUrl && (
              <span className="hover:text-foreground hover:underline cursor-pointer">
                Politique de confidentialité
              </span>
            )}
            {privacyUrl && (termsUrl || supportUrl) && <span className="text-muted-foreground/30">•</span>}
            {termsUrl && (
              <span className="hover:text-foreground hover:underline cursor-pointer">
                Conditions d'utilisation
              </span>
            )}
            {termsUrl && supportUrl && <span className="text-muted-foreground/30">•</span>}
            {supportUrl && (
              <span className="hover:text-foreground hover:underline cursor-pointer">
                Support
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
