import { Outlet } from '@tanstack/react-router';
import { AccountSidebar } from './AccountSidebar';
import { AccountTopBar } from './AccountTopBar';

export function AccountLayout() {
  return (
    <div className="min-h-screen bg-background">
      <AccountTopBar />
      <div className="mx-auto flex min-h-[calc(100vh-3.5rem)] w-full max-w-6xl">
        <aside className="hidden w-60 shrink-0 border-r border-border md:block">
          <AccountSidebar />
        </aside>
        <main className="min-w-0 flex-1 px-4 py-8 md:px-10 md:py-12">
          <div className="mx-auto w-full max-w-2xl">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
