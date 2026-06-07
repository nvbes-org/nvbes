import { Link } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';

export function LandingPageHeader() {
  return (
    <header className="mx-auto flex max-w-7xl items-center justify-between px-6 py-6 lg:px-10">
      <Link className="text-3xl font-semibold tracking-normal" to="/">
        nvbes
      </Link>
      <nav className="hidden items-center gap-12 text-sm text-zinc-700 md:flex">
        {['Platform', 'Security', 'Pricing', 'Docs'].map((item) => (
          <a href={`#${item.toLowerCase()}`} key={item}>
            {item}
          </a>
        ))}
      </nav>
      <Button
        asChild
        className="border-zinc-300 bg-white text-zinc-950 hover:bg-zinc-50"
        variant="outline"
      >
        <Link to="/login">Sign in</Link>
      </Button>
    </header>
  );
}
