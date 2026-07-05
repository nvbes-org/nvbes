import { InvisibleUnicodeWarning } from '@nvbes/web-runtime';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, ShieldCheck } from 'lucide-react';
import { useEffect, useState } from 'react';
import {
  getDeveloperConsentScreen,
  listDeveloperOAuthClients,
  upsertDeveloperConsentScreen,
  type DeveloperConsentScreenForm,
} from '../developer.api';
import { ConsentScreenPreview } from './ConsentScreenPreview';

const emptyForm: DeveloperConsentScreenForm = {
  productName: '',
  description: '',
  logoUrl: '',
  supportUrl: '',
  privacyUrl: '',
  termsUrl: '',
  brandColor: '',
  customCss: '',
  helpText: '',
};

export function ConsentScreenPage() {
  const queryClient = useQueryClient();
  const clientsQuery = useQuery({
    queryKey: ['developer-oauth-clients'],
    queryFn: ({ signal }) => listDeveloperOAuthClients(signal),
    staleTime: 30_000,
  });
  const [clientId, setClientId] = useState('');
  const selectedClientId = clientId || clientsQuery.data?.[0]?.client_id || '';
  const consentQuery = useQuery({
    queryKey: ['developer-consent-screen', selectedClientId],
    queryFn: ({ signal }) => getDeveloperConsentScreen(selectedClientId, signal),
    enabled: selectedClientId.length > 0,
    staleTime: 30_000,
  });
  const [form, setForm] = useState<DeveloperConsentScreenForm>(emptyForm);
  const upsertMutation = useMutation({
    mutationFn: () => upsertDeveloperConsentScreen(selectedClientId, form),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['developer-consent-screen', selectedClientId] }),
  });

  useEffect(() => {
    if (!consentQuery.data) {
      return;
    }
    setForm({
      productName: consentQuery.data.product_name,
      description: consentQuery.data.description,
      logoUrl: consentQuery.data.logo_url ?? '',
      supportUrl: consentQuery.data.support_url ?? '',
      privacyUrl: consentQuery.data.privacy_url ?? '',
      termsUrl: consentQuery.data.terms_url ?? '',
      brandColor: consentQuery.data.brand_color ?? '',
      customCss: consentQuery.data.custom_css ?? '',
      helpText: consentQuery.data.help_text ?? '',
    });
  }, [consentQuery.data]);

  if (clientsQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (clientsQuery.isError || !clientsQuery.data) {
    return <ConsentUnavailable />;
  }

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <ShieldCheck className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Consent screens</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Client-specific product identity, support, privacy, terms, and consent copy.
          </p>
        </div>
      </div>
      <div className="grid gap-4 lg:grid-cols-[250px_1fr]">
        <aside className="rounded-lg border border-border bg-card p-4 h-fit">
          <label className="grid gap-2 text-sm font-medium">
            OAuth client
            <select
              className="h-10 rounded-md border border-input bg-background px-3 text-sm"
              value={selectedClientId}
              onChange={(event) => setClientId(event.currentTarget.value)}
            >
              {clientsQuery.data.map((client) => (
                <option key={client.client_id} value={client.client_id}>
                  {client.name}
                </option>
              ))}
            </select>
          </label>
          <p className="mt-4 text-xs text-muted-foreground">
            {consentQuery.data?.configured ? 'Configured' : 'Not configured'}
          </p>
        </aside>

        <div className="grid gap-4 xl:grid-cols-2">
          {/* Builder Form */}
          <form
            className="rounded-lg border border-border bg-card p-5 h-fit"
            onSubmit={(event) => {
              event.preventDefault();
              upsertMutation.mutate();
            }}
          >
            <h3 className="mb-4 text-sm font-semibold text-foreground">Éditeur</h3>
            <div className="grid gap-4 md:grid-cols-2">
              <TextField
                label="Product name"
                value={form.productName}
                onChange={(productName) => setForm((current) => ({ ...current, productName }))}
              />
              <TextField
                label="Logo URL"
                value={form.logoUrl}
                onChange={(logoUrl) => setForm((current) => ({ ...current, logoUrl }))}
              />
              <TextField
                label="Support URL"
                value={form.supportUrl}
                onChange={(supportUrl) => setForm((current) => ({ ...current, supportUrl }))}
              />
              <TextField
                label="Privacy URL"
                value={form.privacyUrl}
                onChange={(privacyUrl) => setForm((current) => ({ ...current, privacyUrl }))}
              />
              <TextField
                label="Terms URL"
                value={form.termsUrl}
                onChange={(termsUrl) => setForm((current) => ({ ...current, termsUrl }))}
              />
              <label className="grid gap-2 text-sm font-medium">
                Brand color
                <div className="flex gap-2 items-center">
                  <input
                    type="color"
                    value={
                      form.brandColor.startsWith('#') && form.brandColor.length === 7
                        ? form.brandColor
                        : '#3b82f6'
                    }
                    onChange={(e) =>
                      setForm((current) => ({ ...current, brandColor: e.currentTarget.value }))
                    }
                    className="h-10 w-12 rounded-md border border-input bg-background p-1 cursor-pointer"
                  />
                  <input
                    className="h-10 flex-1 rounded-md border border-input bg-background px-3 text-sm"
                    value={form.brandColor}
                    placeholder="#3b82f6"
                    onChange={(event) =>
                      setForm((current) => ({ ...current, brandColor: event.currentTarget.value }))
                    }
                  />
                </div>
              </label>
            </div>

            <label className="mt-4 grid gap-2 text-sm font-medium">
              Description
              <textarea
                className="min-h-20 rounded-md border border-input bg-background px-3 py-2 text-sm"
                value={form.description}
                onChange={(event) =>
                  setForm((current) => ({ ...current, description: event.currentTarget.value }))
                }
              />
              <InvisibleUnicodeWarning value={form.description} />
            </label>

            <div className="grid gap-4 md:grid-cols-2 mt-4">
              <label className="grid gap-2 text-sm font-medium">
                Help text
                <textarea
                  className="min-h-24 rounded-md border border-input bg-background px-3 py-2 text-sm"
                  value={form.helpText}
                  placeholder="Ex: Si vous rencontrez un problème, contactez le support."
                  onChange={(event) =>
                    setForm((current) => ({ ...current, helpText: event.currentTarget.value }))
                  }
                />
                <InvisibleUnicodeWarning value={form.helpText} />
              </label>
              <label className="grid gap-2 text-sm font-medium">
                Custom CSS
                <textarea
                  className="min-h-24 font-mono rounded-md border border-input bg-background px-3 py-2 text-sm"
                  value={form.customCss}
                  placeholder="Ex: button { border-radius: 9999px; }"
                  onChange={(event) =>
                    setForm((current) => ({ ...current, customCss: event.currentTarget.value }))
                  }
                />
                <InvisibleUnicodeWarning value={form.customCss} />
              </label>
            </div>

            <button
              className="mt-5 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50 transition-opacity hover:opacity-90"
              disabled={upsertMutation.isPending || selectedClientId.length === 0}
              type="submit"
            >
              Save consent screen
            </button>
          </form>

          {/* Interactive Live Preview */}
          <div className="rounded-lg border border-border bg-card p-5 flex flex-col h-fit min-h-[450px]">
            <h3 className="mb-4 text-sm font-semibold text-foreground">Aperçu en direct</h3>
            <div className="flex flex-1 items-center justify-center">
              <div className="w-full max-w-sm">
                <ConsentScreenPreview
                  productName={form.productName}
                  description={form.description}
                  logoUrl={form.logoUrl}
                  supportUrl={form.supportUrl}
                  privacyUrl={form.privacyUrl}
                  termsUrl={form.termsUrl}
                  brandColor={form.brandColor}
                  customCss={form.customCss}
                  helpText={form.helpText}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

function TextField(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  return (
    <label className="grid gap-2 text-sm font-medium">
      {props.label}
      <input
        className="h-10 rounded-md border border-input bg-background px-3 text-sm"
        value={props.value}
        placeholder={props.placeholder}
        onChange={(event) => props.onChange(event.currentTarget.value)}
      />
      <InvisibleUnicodeWarning value={props.value} />
    </label>
  );
}

function ConsentUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Consent screens unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load OAuth clients.
      </p>
    </section>
  );
}
