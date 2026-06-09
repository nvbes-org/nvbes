import { Button } from '@/components/ui/button';
import { navItems } from './LandingPage.product.data';

export function ProductMockupSidebar() {
  return (
    <aside className="flex flex-col bg-zinc-950 p-4 text-white">
      <div className="text-xl font-semibold tracking-normal">nvbes</div>
      <Button
        type="button"
        variant="outline"
        className="mt-6 justify-between border-white/10 bg-white/5 text-left text-xs text-zinc-200 hover:bg-white/10 hover:text-white"
      >
        Acme Corp <span className="text-zinc-500">⌄</span>
      </Button>
      <nav className="mt-4 space-y-1 text-sm text-zinc-300">
        {navItems.map((item, index) => (
          <div
            className={`rounded-md px-3 py-2 ${index === 0 ? 'bg-white/12 text-white' : ''}`}
            key={item}
          >
            {item}
          </div>
        ))}
      </nav>
      <div className="mt-auto rounded-md border border-white/10 bg-white/5 p-3 text-xs">
        <div className="font-medium text-white">Sam Dovis</div>
        <div className="mt-1 text-zinc-500">s.dovis@acme.co</div>
      </div>
    </aside>
  );
}
