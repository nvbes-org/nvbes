import { quickstarts, type QuickstartKey } from '@/developer.quickstarts';

async function copy(value: string) {
  await navigator.clipboard.writeText(value);
}

function CodeBlock({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-card">
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">{label}</h2>
        <button type="button" className="text-sm text-primary" onClick={() => void copy(value)}>
          Copy
        </button>
      </div>
      <pre className="overflow-auto p-4 text-xs leading-6">
        <code>{value}</code>
      </pre>
    </div>
  );
}

export function QuickstartPage({ quickstartKey }: { quickstartKey: QuickstartKey }) {
  const quickstart = quickstarts[quickstartKey];

  return (
    <section className="mx-auto grid max-w-5xl gap-6 px-6 py-10">
      <header>
        <h1 className="text-3xl font-semibold md:text-5xl">{quickstart.title}</h1>
        <p className="mt-4 max-w-2xl text-sm leading-6 text-muted-foreground">
          Start with the SDK command, then wire the minimal integration code.
        </p>
      </header>
      <CodeBlock label="Install" value={quickstart.command} />
      <CodeBlock label="Code" value={quickstart.code} />
    </section>
  );
}
