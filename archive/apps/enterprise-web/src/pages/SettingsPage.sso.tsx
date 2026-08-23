import type {
  CreateFederatedIdentityProviderInput,
  EnterpriseTrustCenterResponse,
} from '@nvbes/identity-client';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { KeyRound } from 'lucide-react';
import { useState } from 'react';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';
import { enterpriseClient } from '../enterprise.api';
import { enterpriseQueryKeys } from '../enterprise.queries';
import { EmptyState } from './SettingsPage.empty';

type TrustProvider = EnterpriseTrustCenterResponse['sso']['providers'][number];
type ProviderType = 'oidc' | 'saml';
type ProviderFamily = 'okta' | 'azure_ad' | 'google_workspace' | 'custom';

type ProviderForm = {
  providerType: ProviderType;
  providerFamily: ProviderFamily;
  name: string;
  clientId: string;
  issuer: string;
  metadataUrl: string;
};

const initialForm: ProviderForm = {
  providerType: 'oidc',
  providerFamily: 'okta',
  name: '',
  clientId: '',
  issuer: '',
  metadataUrl: '',
};

export function SsoCard({ tenantId, providers }: { tenantId: string; providers: TrustProvider[] }) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<ProviderForm>(initialForm);
  const createMutation = useMutation({
    mutationFn: (input: CreateFederatedIdentityProviderInput) =>
      enterpriseClient.createFederatedIdentityProvider(tenantId, input),
    onSuccess: () => {
      setForm(initialForm);
      return queryClient.invalidateQueries({ queryKey: enterpriseQueryKeys.trustCenter });
    },
  });
  const disabled = !form.name.trim() || !form.clientId.trim() || !hasProviderSource(form);

  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader className="border-b border-border">
        <CardTitle>SSO providers</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <form
          className="grid gap-3 md:grid-cols-2"
          onSubmit={(event) => {
            event.preventDefault();
            if (!disabled) {
              createMutation.mutate(providerInput(form));
            }
          }}
        >
          <div className="grid gap-2">
            <Label htmlFor="sso-name">Name</Label>
            <Input
              id="sso-name"
              value={form.name}
              placeholder="Okta production"
              onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
            />
          </div>
          <div className="grid gap-2">
            <Label>Protocol</Label>
            <Select
              value={form.providerType}
              onValueChange={(value) =>
                setForm((current) => ({
                  ...current,
                  providerFamily: value === 'saml' ? 'custom' : current.providerFamily,
                  providerType: value as ProviderType,
                }))
              }
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="oidc">OIDC</SelectItem>
                <SelectItem value="saml">SAML</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-2">
            <Label>Provider</Label>
            <Select
              value={form.providerFamily}
              onValueChange={(value) =>
                setForm((current) => ({ ...current, providerFamily: value as ProviderFamily }))
              }
              disabled={form.providerType === 'saml'}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="okta">Okta</SelectItem>
                <SelectItem value="azure_ad">Azure AD</SelectItem>
                <SelectItem value="google_workspace">Google Workspace</SelectItem>
                <SelectItem value="custom">Custom</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-2">
            <Label htmlFor="sso-client-id">
              {form.providerType === 'saml' ? 'SP entity ID' : 'Client ID'}
            </Label>
            <Input
              id="sso-client-id"
              value={form.clientId}
              onChange={(event) =>
                setForm((current) => ({ ...current, clientId: event.target.value }))
              }
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="sso-issuer">Issuer</Label>
            <Input
              id="sso-issuer"
              value={form.issuer}
              placeholder="https://idp.example.com"
              onChange={(event) =>
                setForm((current) => ({ ...current, issuer: event.target.value }))
              }
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="sso-metadata-url">Metadata URL</Label>
            <Input
              id="sso-metadata-url"
              value={form.metadataUrl}
              placeholder="https://idp.example.com/metadata"
              onChange={(event) =>
                setForm((current) => ({ ...current, metadataUrl: event.target.value }))
              }
            />
          </div>
          <Button type="submit" disabled={disabled || createMutation.isPending}>
            <KeyRound className="size-4" />
            Add provider
          </Button>
        </form>

        {createMutation.error ? <MutationError error={createMutation.error} /> : null}

        {providers.length > 0 ? <ProviderTable providers={providers} /> : <NoProviderState />}
      </CardContent>
    </Card>
  );
}

function ProviderTable({ providers }: { providers: TrustProvider[] }) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead>Type</TableHead>
          <TableHead>Status</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {providers.map((provider) => (
          <TableRow key={provider.id}>
            <TableCell className="font-medium">{provider.name}</TableCell>
            <TableCell>{provider.provider_family}</TableCell>
            <TableCell>
              <Badge
                variant={provider.status === 'active' ? 'default' : 'secondary'}
                className="rounded-md"
              >
                {provider.status}
              </Badge>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function NoProviderState() {
  return (
    <EmptyState
      title="No SSO provider"
      description="Configure a SAML or OIDC provider to centralize enterprise login."
    />
  );
}

function hasProviderSource(form: ProviderForm): boolean {
  if (form.providerType === 'saml') {
    return Boolean(form.issuer.trim() && form.metadataUrl.trim());
  }
  return Boolean(form.issuer.trim() || form.metadataUrl.trim());
}

function providerInput(form: ProviderForm): CreateFederatedIdentityProviderInput {
  return {
    provider_type: form.providerType,
    provider_family: form.providerType === 'saml' ? 'custom' : form.providerFamily,
    name: form.name.trim(),
    client_id: form.clientId.trim(),
    issuer: optionalValue(form.issuer),
    metadata_url: optionalValue(form.metadataUrl),
    status: 'active',
  };
}

function optionalValue(value: string): string | undefined {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function MutationError({ error }: { error: unknown }) {
  return (
    <p className="text-sm text-destructive">
      {error instanceof Error ? error.message : 'The SSO provider action failed.'}
    </p>
  );
}
