import { ArrowRight, Globe2, LockKeyhole } from 'lucide-react';
import { CheckList } from './LandingPage.ui';

export function LandingPageSecurity() {
  return (
    <section
      className="mx-auto grid max-w-7xl gap-12 px-6 py-16 lg:grid-cols-[0.8fr_1fr_1fr] lg:px-10"
      id="security"
    >
      <div>
        <h2 className="text-3xl font-semibold tracking-normal">
          Security by design. Compliance by default.
        </h2>
        <p className="mt-5 leading-7 text-zinc-600">
          nvbes is built on a security-first architecture with encryption at every layer and privacy
          controls you can rely on.
        </p>
        <a
          className="mt-7 inline-flex items-center gap-2 text-sm font-medium text-teal-800"
          href="#security"
        >
          View security overview <ArrowRight className="size-4" />
        </a>
      </div>
      <div className="border-l border-zinc-200 pl-8">
        <LockKeyhole className="size-6 text-zinc-950" />
        <h3 className="mt-5 font-semibold">Audit you can trust</h3>
        <p className="mt-3 text-sm leading-6 text-zinc-600">
          Every action is recorded, immutable, and exportable.
        </p>
        <CheckList
          items={[
            'Real-time audit stream',
            'Tamper-evident logs',
            'Export to SIEM via API',
            'Retention policies & holds',
          ]}
        />
      </div>
      <div className="border-l border-zinc-200 pl-8">
        <Globe2 className="size-6 text-zinc-950" />
        <h3 className="mt-5 font-semibold">Data residency & compliance</h3>
        <p className="mt-3 text-sm leading-6 text-zinc-600">
          Your data stays in the region you choose with industry-leading compliance.
        </p>
        <CheckList
          items={[
            'Region selection (US, EU, APAC)',
            'SOC 2 Type II compliant',
            'GDPR & CCPA aligned',
            'DPA available',
          ]}
        />
      </div>
    </section>
  );
}
