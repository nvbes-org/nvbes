import { InvisibleUnicodeWarning } from '@nvbes/web-runtime';
import { ClipboardButton } from '@nvbes/web-ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import {
  createDeveloperWebhook,
  deleteDeveloperWebhook,
  listDeveloperWebhooks,
} from '@/developer.api';
import {
  EmptyState,
  Field,
  PageHeader,
  buttonClass,
  formString,
  inputClass,
} from './Portal.shared';

const eventOptions = ['user.created', 'login.failed', 'session.revoked', 'client.created'] as const;

export function PortalWebhooksPage() {
  const queryClient = useQueryClient();
  const [signingSecret, setSigningSecret] = useState<string | null>(null);
  const [endpointUrl, setEndpointUrl] = useState('');
  const webhooksQuery = useQuery({
    queryKey: ['developer', 'webhooks'],
    queryFn: ({ signal }) => listDeveloperWebhooks({ signal }),
  });
  const createWebhook = useMutation({
    mutationFn: createDeveloperWebhook,
    onSuccess: (result) => {
      setSigningSecret(result.signing_secret);
      void queryClient.invalidateQueries({ queryKey: ['developer', 'webhooks'] });
    },
  });
  const deleteWebhook = useMutation({
    mutationFn: deleteDeveloperWebhook,
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ['developer', 'webhooks'] }),
  });

  return (
    <section>
      <PageHeader
        title="Webhooks"
        body="Subscribe endpoints to Identity lifecycle events and copy the signing secret once."
      />
      <form
        className="mb-6 grid gap-3 rounded-md border border-border bg-card p-4 md:grid-cols-2"
        onSubmit={(event) => {
          event.preventDefault();
          const form = new FormData(event.currentTarget);
          createWebhook.mutate({
            name: formString(form, 'name').trim(),
            url: formString(form, 'url').trim(),
            events: eventOptions.filter((option) => form.getAll('events').includes(option)),
          });
        }}
      >
        <Field label="Name">
          <input name="name" className={inputClass} required />
        </Field>
        <Field label="Endpoint URL">
          <input
            name="url"
            className={inputClass}
            value={endpointUrl}
            onChange={(event) => setEndpointUrl(event.currentTarget.value)}
            required
          />
          <InvisibleUnicodeWarning value={endpointUrl} />
        </Field>
        <fieldset className="grid gap-2 text-sm">
          <legend className="font-medium">Events</legend>
          {eventOptions.map((eventType) => (
            <label key={eventType} className="flex items-center gap-2">
              <input name="events" type="checkbox" value={eventType} />
              {eventType}
            </label>
          ))}
        </fieldset>
        <button type="submit" className={buttonClass} disabled={createWebhook.isPending}>
          Create webhook
        </button>
      </form>
      {signingSecret ? (
        <div className="mb-6 rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-semibold">Signing secret</h2>
          <div className="mt-2 flex items-start gap-2 rounded-md bg-background p-3">
            <code className="min-w-0 flex-1 overflow-auto text-xs">{signingSecret}</code>
            <ClipboardButton value={signingSecret} />
          </div>
          <InvisibleUnicodeWarning value={signingSecret} />
        </div>
      ) : null}
      {webhooksQuery.data?.length ? (
        <div className="grid gap-3">
          {webhooksQuery.data.map((endpoint) => (
            <div key={endpoint.id} className="rounded-md border border-border bg-card p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h2 className="font-semibold">{endpoint.name}</h2>
                  <p className="mt-1 break-all text-sm text-muted-foreground">{endpoint.url}</p>
                  <p className="mt-2 text-xs text-muted-foreground">
                    {endpoint.events.join(', ')} · secret ****{endpoint.signing_secret_last4}
                  </p>
                </div>
                <button
                  type="button"
                  className="text-sm text-destructive"
                  onClick={() => deleteWebhook.mutate(endpoint.id)}
                >
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <EmptyState title="No webhooks" body="Create an endpoint to receive Identity events." />
      )}
    </section>
  );
}
