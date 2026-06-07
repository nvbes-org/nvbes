import { Button } from '@/components/ui/button';
import { auditEvents, files, metrics, policyGroups } from './LandingPage.product.data';

export function ProductMockupContent() {
  return (
    <div className="min-w-0 p-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold text-zinc-950">Overview</h2>
          <p className="text-xs text-zinc-500">Acme Corp · Pro Plan</p>
        </div>
        <div className="rounded-md border border-emerald-200 bg-emerald-50 px-3 py-1.5 text-xs text-emerald-800">
          All systems operational
        </div>
      </div>

      <div className="mt-6 grid min-w-0 gap-3 md:grid-cols-4">
        {metrics.map(([label, value, note]) => (
          <div className="rounded-md border border-zinc-200 p-4" key={label}>
            <div className="text-xs text-zinc-500">{label}</div>
            <div className="mt-2 text-2xl font-semibold text-zinc-950">{value}</div>
            <div className="mt-1 text-xs text-zinc-400">{note}</div>
          </div>
        ))}
      </div>

      <div className="mt-5 grid min-w-0 gap-5 xl:grid-cols-[minmax(0,1fr)_230px]">
        <section className="rounded-md border border-zinc-200 p-4">
          <div className="mb-4 flex items-center justify-between">
            <div>
              <h3 className="font-medium text-zinc-950">Workspace files</h3>
              <p className="text-xs text-zinc-500">/ product / q2-launch</p>
            </div>
            <Button className="bg-teal-700 hover:bg-teal-800" size="sm">
              Upload
            </Button>
          </div>
          <div className="space-y-1 text-sm">
            {files.map(([name, size, updated]) => (
              <div
                className="grid min-w-0 grid-cols-[minmax(0,1fr)_56px_112px] border-t border-zinc-100 py-2"
                key={name}
              >
                <span className="font-medium text-zinc-800">{name}</span>
                <span className="text-zinc-500">{size}</span>
                <span className="text-zinc-500">{updated}</span>
              </div>
            ))}
          </div>
        </section>

        <section className="rounded-md border border-zinc-200 p-4">
          <h3 className="font-medium text-zinc-950">Access policy</h3>
          <p className="mt-1 text-xs text-zinc-500">q2-launch</p>
          <div className="mt-4 space-y-3 text-sm">
            {policyGroups.map((name) => (
              <div className="flex items-center justify-between" key={name}>
                <span className="text-zinc-700">{name}</span>
                <span className="text-xs text-teal-700">Edit</span>
              </div>
            ))}
          </div>
          <div className="mt-5 border-t border-zinc-100 pt-4 text-xs text-zinc-500">
            MFA required · Link sharing off · Watermark on
          </div>
        </section>
      </div>

      <section className="mt-5 rounded-md border border-zinc-200 p-4">
        <div className="flex items-center justify-between">
          <h3 className="font-medium text-zinc-950">Recent audit events</h3>
          <span className="text-xs text-teal-700">View all</span>
        </div>
        <div className="mt-3 space-y-2 text-sm">
          {auditEvents.map(([event, time]) => (
            <div
              className="flex items-center justify-between border-t border-zinc-100 pt-2"
              key={event}
            >
              <span className="text-zinc-700">{event}</span>
              <span className="text-xs text-zinc-500">{time}</span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
