import { Link } from '@tanstack/react-router';
import { AuthFooterLink } from '@/components/AuthFooterLink';

const LEGAL_LINKS: ReadonlyArray<{ to: string; label: string }> = [
  { to: '/legal/site-legal-notice', label: 'Mentions légales' },
  { to: '/legal/privacy-policy', label: 'Confidentialité' },
  { to: '/legal/terms-of-service', label: 'CGV' },
];

export function LoginPageFooter({ searchStr }: { searchStr: string }) {
  return (
    <AuthFooterLink prompt="Pas de compte ?" to={`/register${searchStr}`} label="S'inscrire" />
  );
}

export function LoginPageLegalLinks() {
  return (
    <nav
      aria-label="Liens légaux"
      className="mt-5 flex flex-wrap justify-center gap-x-3 gap-y-1 text-center"
    >
      {LEGAL_LINKS.map((link) => (
        <Link
          key={link.to}
          to={link.to}
          className="text-xs text-muted-foreground hover:text-foreground hover:underline"
        >
          {link.label}
        </Link>
      ))}
    </nav>
  );
}
