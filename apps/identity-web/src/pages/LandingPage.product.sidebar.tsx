import { navItems } from './LandingPage.product.data';

export function ProductMockupSidebar() {
  return (
    <aside className="flex flex-col bg-zinc-950 p-4 text-white">
      <div className="text-xl font-semibold tracking-normal">nvbes</div>
      <button className="mt-6 flex items-center justify-between rounded-md border border-white/10 bg-white/5 px-3 py-2 text-left text-xs text-zinc-200">
        Acme Corp <span className="text-zinc-500">⌄</span>
      </button>
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
