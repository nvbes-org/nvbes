const LEGAL_LINKS = [
  { href: '/legal/site-legal-notice', label: 'Mentions légales' },
  { href: '/legal/privacy-policy', label: 'Confidentialité' },
  { href: '/legal/terms-of-service', label: 'Conditions' },
  { href: '/legal/data-processing-agreement', label: 'DPA' },
  { href: '/legal/subprocessors', label: 'Sous-traitants' },
  { href: '/legal/data-retention-policy', label: 'Conservation' },
  { href: '/legal/service-level-agreement', label: 'SLA' },
  { href: '/legal/commercial-billing-policy', label: 'Facturation' },
] as const;

export function LoginPageFooter({ searchStr }: { searchStr: string }) {
  return (
    <p className="text-center text-sm text-muted-foreground">
      Pas de compte ?{' '}
      <a href={`/register${searchStr}`} className="font-medium text-primary hover:underline">
        S&apos;inscrire
      </a>
    </p>
  );
}

export function LoginPageLegalLinks() {
  return (
    <nav
      aria-label="Liens légaux"
      className="mt-5 flex flex-wrap justify-center gap-x-3 gap-y-1 text-center"
    >
      {LEGAL_LINKS.map((link) => (
        <a
          key={link.href}
          href={link.href}
          className="text-xs text-muted-foreground hover:text-foreground hover:underline"
        >
          {link.label}
        </a>
      ))}
    </nav>
  );
}
