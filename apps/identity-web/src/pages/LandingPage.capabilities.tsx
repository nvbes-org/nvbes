import { capabilities } from './LandingPage.content';
import { CheckList } from './LandingPage.ui';

export function LandingPageCapabilities() {
  return (
    <section className="mx-auto max-w-6xl border-y border-zinc-200 py-3" id="platform">
      {capabilities.map(({ title, body, icon: IconComponent, points }) => (
        <div
          className="grid gap-8 border-b border-zinc-200 px-6 py-8 last:border-b-0 md:grid-cols-[130px_1fr_1fr] md:items-center"
          key={title}
        >
          <div className="flex size-20 items-center justify-center rounded-md bg-teal-50 text-teal-800">
            <IconComponent className="size-10" />
          </div>
          <div>
            <h2 className="text-2xl font-semibold tracking-normal">{title}</h2>
            <p className="mt-2 max-w-md leading-7 text-zinc-600">{body}</p>
          </div>
          <CheckList items={points} />
        </div>
      ))}
    </section>
  );
}
