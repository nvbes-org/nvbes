import { Link, Outlet } from '@tanstack/react-router';
import { startDeveloperLogin } from '../developer.auth';

export function DeveloperPublicLayout() {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card">
        <nav className="mx-auto flex h-14 max-w-7xl items-center justify-between px-4 md:px-8">
          <Link to="/" className="font-heading text-base font-semibold">
            nvbes Developers
          </Link>
          <div className="flex items-center gap-4 text-sm">
            <Link to="/quickstarts/react">Quickstarts</Link>
            <Link to="/api-reference">API Reference</Link>
            <button
              type="button"
              onClick={() => void startDeveloperLogin(`${window.location.origin}/portal/apps`)}
            >
              Sign in
            </button>
            <Link to="/portal/apps" className="font-medium text-primary">
              Portal
            </Link>
          </div>
        </nav>
      </header>
      <Outlet />
    </main>
  );
}
