import {
  type AccountEntry,
  type AccountMe,
  type AccountPrincipal,
  type AccountWorkspace,
  identityClient,
} from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Calendar, Mail, MapPin, User } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { z } from 'zod';
import { accountQueryKeys } from '@/account.queries';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { identityHttpClient } from '../identity.http';

const UpdateProfileSchema = z.object({
  user: z.object({
    id: z.string(),
    email: z.string(),
    display_name: z.string(),
    firstname: z.string().nullable().optional(),
    lastname: z.string().nullable().optional(),
    username: z.string().nullable().optional(),
    birthdate: z.string().nullable().optional(),
    region: z.string().nullable().optional(),
    email_verified: z.boolean(),
    mfa_enabled: z.boolean(),
    created_at: z.string(),
  }),
});

interface UpdateProfileInput {
  firstname: string | null;
  lastname: string | null;
  username: string | null;
  birthdate: string | null;
  region: string | null;
}

function updateProfile(input: UpdateProfileInput) {
  return identityHttpClient.request('/auth/me', UpdateProfileSchema, {
    method: 'PATCH',
    body: {
      firstname: input.firstname || undefined,
      lastname: input.lastname || undefined,
      username: input.username || undefined,
      birthdate: input.birthdate || undefined,
      region: input.region || undefined,
    },
  });
}

function PersonalInfoSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-48" />
        <Skeleton className="h-4 w-72 mt-1" />
      </div>
      <Card>
        <CardContent className="flex flex-col gap-3 pt-6">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-10 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

function SuccessMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-sm text-emerald-600">
      {message}
    </div>
  );
}

export default function AccountPersonalInfoPage() {
  const queryClient = useQueryClient();
  const didInitializeFormRef = useRef(false);

  const [firstname, setFirstname] = useState('');
  const [lastname, setLastname] = useState('');
  const [username, setUsername] = useState('');
  const [birthdate, setBirthdate] = useState('');
  const [region, setRegion] = useState('');

  const [editError, setEditError] = useState<string | null>(null);
  const [editSuccess, setEditSuccess] = useState(false);

  const personalInfoQueryKey = accountQueryKeys.personalInfo;

  const { data: user } = useQuery({
    queryKey: personalInfoQueryKey,
    queryFn: ({ signal }) => identityClient.getMe({ signal }).then((me) => me.user),
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const selectedUser = user ?? null;

  useEffect(() => {
    if (!selectedUser || didInitializeFormRef.current) return;
    setFirstname(selectedUser.firstname ?? '');
    setLastname(selectedUser.lastname ?? '');
    setUsername(selectedUser.username ?? '');
    setBirthdate(selectedUser.birthdate ?? '');
    setRegion(selectedUser.region ?? '');
    didInitializeFormRef.current = true;
  }, [selectedUser]);

  const mutation = useMutation({
    mutationFn: (input: UpdateProfileInput) => updateProfile(input),
    onSuccess: (data) => {
      queryClient.setQueryData<AccountPrincipal>(personalInfoQueryKey, data.user);
      queryClient.setQueryData<{
        me: AccountMe | null;
        workspaces: AccountWorkspace[];
        accounts: AccountEntry[];
      }>(accountQueryKeys.context, (current) =>
        current?.me
          ? {
              ...current,
              me: {
                ...current.me,
                user: data.user,
              },
            }
          : current,
      );
      didInitializeFormRef.current = true;
      setFirstname(data.user.firstname ?? '');
      setLastname(data.user.lastname ?? '');
      setUsername(data.user.username ?? '');
      setBirthdate(data.user.birthdate ?? '');
      setRegion(data.user.region ?? '');
      setEditSuccess(true);
      setEditError(null);
      queryClient.invalidateQueries({ queryKey: accountQueryKeys.all });
    },
    onError: () => {
      setEditError(
        'La mise a jour du profil a echoue. Verifiez les champs ou reessayez plus tard.',
      );
      setEditSuccess(false);
    },
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setEditError(null);
    setEditSuccess(false);

    mutation.mutate({
      firstname: firstname.trim() || null,
      lastname: lastname.trim() || null,
      username: username.trim() || null,
      birthdate: birthdate || null,
      region: region || null,
    });
  };

  if (!selectedUser) return <PersonalInfoSkeleton />;

  const fullName =
    [selectedUser.firstname, selectedUser.lastname].filter(Boolean).join(' ') ||
    selectedUser.display_name;
  const memberSince = new Date(selectedUser.created_at).toLocaleDateString('fr-FR', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Informations personnelles</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Consultez et gerez vos informations de profil.
        </p>
      </div>

      <div className="flex flex-col gap-6">
        <Card>
          <CardHeader>
            <CardTitle>Profil</CardTitle>
            <CardDescription>Vos informations d&apos;identite</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-1">
            <div className="flex items-center gap-3 py-1">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <User className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-xs text-muted-foreground">Nom complet</span>
                <span className="text-sm font-medium truncate">{fullName}</span>
              </div>
            </div>
            <Separator className="my-1" />
            <div className="flex items-center gap-3 py-1">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <Mail className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-xs text-muted-foreground">Email</span>
                <span className="text-sm font-medium truncate">{selectedUser.email}</span>
              </div>
            </div>
            <div className="flex items-center justify-between py-1">
              <span className="text-sm font-medium text-muted-foreground">Statut</span>
              <Badge variant={selectedUser.email_verified ? 'default' : 'secondary'}>
                {selectedUser.email_verified ? 'Verifie' : 'En attente'}
              </Badge>
            </div>
            <Separator className="my-1" />
            <div className="flex items-center gap-3 py-1">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <MapPin className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-xs text-muted-foreground">Region</span>
                <span className="text-sm font-medium truncate">
                  {selectedUser.region ?? 'Non definie'}
                </span>
              </div>
            </div>
            <Separator className="my-1" />
            <div className="flex items-center gap-3 py-1">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <Calendar className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-xs text-muted-foreground">Membre depuis</span>
                <span className="text-sm font-medium truncate">{memberSince}</span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Modifier le profil</CardTitle>
            <CardDescription>Mettez a jour vos informations personnelles.</CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmit} className="flex flex-col gap-5">
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="firstname">Prenom</Label>
                  <Input
                    id="firstname"
                    placeholder="Prenom"
                    value={firstname}
                    onChange={(e) => setFirstname(e.target.value)}
                  />
                </div>
                <div className="flex flex-col gap-2">
                  <Label htmlFor="lastname">Nom</Label>
                  <Input
                    id="lastname"
                    placeholder="Nom"
                    value={lastname}
                    onChange={(e) => setLastname(e.target.value)}
                  />
                </div>
              </div>

              <div className="flex flex-col gap-2">
                <Label htmlFor="username">Nom d&apos;utilisateur</Label>
                <Input
                  id="username"
                  placeholder="nomutilisateur"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                />
              </div>

              <div className="flex flex-col gap-2">
                <Label htmlFor="birthdate">Date de naissance</Label>
                <Input
                  id="birthdate"
                  type="date"
                  value={birthdate}
                  onChange={(e) => setBirthdate(e.target.value)}
                />
              </div>

              <div className="flex flex-col gap-2">
                <Label htmlFor="region">Region</Label>
                <Input
                  id="region"
                  placeholder="FR"
                  value={region}
                  onChange={(e) => setRegion(e.target.value)}
                />
              </div>

              {editError && <ErrorMessage message={editError} />}
              {editSuccess && (
                <SuccessMessage message="Votre profil a ete mis a jour avec succes." />
              )}

              <Button type="submit" disabled={mutation.isPending} className="w-full">
                {mutation.isPending ? 'Enregistrement...' : 'Enregistrer les modifications'}
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
