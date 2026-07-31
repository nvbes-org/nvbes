export function AccountPage({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-8">
      <header className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight text-foreground">{title}</h1>
        <p className="max-w-xl text-sm leading-6 text-muted-foreground">{description}</p>
      </header>
      {children}
    </section>
  );
}
