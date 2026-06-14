import { useQuery } from '@tanstack/react-query';

type OpenApiDocument = {
  info?: {
    title?: string;
    version?: string;
  };
  paths?: Record<string, unknown>;
};

async function fetchOpenApi(): Promise<OpenApiDocument> {
  const response = await fetch('/api/openapi.json', { credentials: 'include' });
  if (!response.ok) {
    throw new Error('OpenAPI document unavailable');
  }

  return (await response.json()) as OpenApiDocument;
}

export function ApiReferencePage() {
  const openApiQuery = useQuery({
    queryKey: ['developer', 'openapi'],
    queryFn: fetchOpenApi,
    retry: false,
  });
  const hasApiError = openApiQuery.isError;
  const paths = Object.keys(openApiQuery.data?.paths ?? {});
  const developerPaths = paths.filter((path) => path.startsWith('/developer'));

  return (
    <section className="mx-auto grid max-w-5xl gap-6 px-6 py-10">
      <header>
        <h1 className="text-3xl font-semibold md:text-5xl">
          {openApiQuery.data?.info?.title ?? 'Identity API reference'}
        </h1>
        <p className="mt-4 text-sm text-muted-foreground">
          Version {openApiQuery.data?.info?.version ?? '-'} · {paths.length} paths
        </p>
        <a className="mt-3 inline-flex text-sm font-medium text-primary" href="/docs">
          Open Swagger UI
        </a>
      </header>
      {hasApiError ? (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 p-4">
          <h2 className="text-sm font-semibold text-destructive">API connection error</h2>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            OpenAPI metadata could not be loaded from the Identity API. Start the API service or
            configure the developer-web proxy target, then reload this page.
          </p>
        </div>
      ) : null}
      <div className="rounded-md border border-border bg-card">
        <div className="border-b border-border px-4 py-3 text-sm font-semibold">
          Developer paths
        </div>
        <div className="grid divide-y divide-border">
          {developerPaths.map((path) => (
            <code key={path} className="px-4 py-3 text-sm">
              {path}
            </code>
          ))}
        </div>
      </div>
    </section>
  );
}
