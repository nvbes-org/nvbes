import { ArrowRight } from 'lucide-react';
import { workflow } from './LandingPage.content';

export function LandingPageWorkflow() {
  return (
    <section className="border-b border-zinc-200 bg-zinc-50 py-16">
      <div className="mx-auto grid max-w-7xl gap-12 px-6 lg:grid-cols-[280px_1fr] lg:px-10">
        <div>
          <h2 className="text-3xl font-semibold tracking-normal">
            From access to audit, all in one flow
          </h2>
          <p className="mt-4 leading-7 text-zinc-600">
            A single control plane for your team&apos;s most sensitive workflows.
          </p>
          <a
            className="mt-6 inline-flex items-center gap-2 text-sm font-medium text-teal-800"
            href="#docs"
          >
            View docs <ArrowRight className="size-4" />
          </a>
        </div>
        <div className="grid gap-6 md:grid-cols-5">
          {workflow.map(({ title, body, icon: IconComponent }, index) => (
            <div className="relative" key={title}>
              {index < workflow.length - 1 ? (
                <div className="absolute left-14 top-9 hidden h-px w-full border-t border-dashed border-zinc-400 md:block" />
              ) : null}
              <div className="relative flex size-18 items-center justify-center rounded-full bg-teal-100 text-teal-900">
                <IconComponent className="size-8" />
              </div>
              <h3 className="mt-5 font-semibold">
                {index + 1}. {title}
              </h3>
              <p className="mt-2 text-sm leading-6 text-zinc-600">{body}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
