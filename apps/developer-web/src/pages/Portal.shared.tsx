export function EmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-md border border-dashed border-border bg-card p-8">
      <h2 className="text-base font-semibold">{title}</h2>
      <p className="mt-2 max-w-xl text-sm text-muted-foreground">{body}</p>
    </div>
  );
}

export function PageHeader({ title, body }: { title: string; body: string }) {
  return (
    <header className="mb-6">
      <h1 className="text-2xl font-semibold">{title}</h1>
      <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">{body}</p>
    </header>
  );
}

export function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="grid gap-1 text-sm font-medium">
      <span>{label}</span>
      {children}
    </label>
  );
}

export const inputClass =
  'h-10 rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-primary';

export const textareaClass =
  'min-h-24 rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary';

export const buttonClass =
  'inline-flex h-10 items-center justify-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground disabled:cursor-not-allowed disabled:opacity-60';

export function formString(form: FormData, key: string, fallback = '') {
  const value = form.get(key);

  return typeof value === 'string' ? value : fallback;
}
