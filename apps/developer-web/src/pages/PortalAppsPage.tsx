import { ClipboardButton } from '@nvbes/web-ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import { createDeveloperApp, listDeveloperApps } from '@/developer.api';
import {
  EmptyState,
  Field,
  PageHeader,
  buttonClass,
  formString,
  inputClass,
  textareaClass,
} from './Portal.shared';

function lines(value: string) {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function PortalAppsPage() {
  const queryClient = useQueryClient();
  const [clientSecret, setClientSecret] = useState<string | null>(null);
  const appsQuery = useQuery({
    queryKey: ['developer', 'apps'],
    queryFn: ({ signal }) => listDeveloperApps({ signal }),
  });
  const createApp = useMutation({
    mutationFn: createDeveloperApp,
    onSuccess: (result) => {
      setClientSecret(result.client_secret);
      void queryClient.invalidateQueries({ queryKey: ['developer', 'apps'] });
    },
  });

  return (
    <section>
      <PageHeader
        title="OAuth apps"
        body="Create apps, copy client IDs, and manage redirect URI configuration."
      />
      <form
        className="mb-6 grid gap-3 rounded-md border border-border bg-card p-4 md:grid-cols-2"
        onSubmit={(event) => {
          event.preventDefault();
          const form = new FormData(event.currentTarget);
          createApp.mutate({
            name: formString(form, 'name').trim(),
            redirect_uris: lines(formString(form, 'redirect_uris')),
            allowed_scopes: lines(formString(form, 'allowed_scopes')),
            allowed_audiences: lines(formString(form, 'allowed_audiences')),
            allowed_resources: lines(formString(form, 'allowed_resources')),
          });
        }}
      >
        <Field label="Name">
          <input name="name" className={inputClass} required />
        </Field>
        <Field label="Allowed scopes">
          <input
            name="allowed_scopes"
            className={inputClass}
            placeholder="openid profile email"
            required
          />
        </Field>
        <Field label="Redirect URIs">
          <textarea name="redirect_uris" className={textareaClass} required />
        </Field>
        <div className="grid gap-3">
          <Field label="Allowed audiences">
            <input name="allowed_audiences" className={inputClass} placeholder="optional" />
          </Field>
          <Field label="Allowed resources">
            <input name="allowed_resources" className={inputClass} placeholder="optional" />
          </Field>
        </div>
        <button type="submit" className={buttonClass} disabled={createApp.isPending}>
          Create app
        </button>
      </form>
      {clientSecret ? (
        <div className="mb-6 rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-semibold">Client secret</h2>
          <div className="mt-2 flex items-start gap-2 rounded-md bg-background p-3">
            <code className="min-w-0 flex-1 overflow-auto text-xs">{clientSecret}</code>
            <ClipboardButton value={clientSecret} />
          </div>
        </div>
      ) : null}
      {appsQuery.data?.length ? (
        <div className="overflow-x-auto rounded-md border border-border bg-card">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-border text-muted-foreground">
              <tr>
                <th className="p-3">Name</th>
                <th className="p-3">Client ID</th>
                <th className="p-3">Type</th>
                <th className="p-3">Redirects</th>
                <th className="p-3">Action</th>
              </tr>
            </thead>
            <tbody>
              {appsQuery.data.map((app) => (
                <tr key={app.id} className="border-b border-border last:border-0">
                  <td className="p-3 font-medium">{app.name}</td>
                  <td className="p-3 font-mono text-xs">{app.client_id}</td>
                  <td className="p-3">{app.client_type}</td>
                  <td className="p-3">{app.redirect_uris.length}</td>
                  <td className="p-3">
                    <ClipboardButton value={app.client_id} className="border-0 text-primary" />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <EmptyState title="No apps" body="Create your first OAuth app to get a client ID." />
      )}
    </section>
  );
}
