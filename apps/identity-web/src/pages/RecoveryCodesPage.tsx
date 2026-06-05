import StepUpForm from '@/components/StepUpForm';
import { generateRecoveryCodes } from '@nvbes/identity-sdk-web';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

export default function RecoveryCodesPage() {
  const navigate = useNavigate();

  const [step, setStep] = useState<'stepup' | 'password' | 'codes'>('stepup');
  const [password, setPassword] = useState('');
  const [codes, setCodes] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleGenerate = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const result = await generateRecoveryCodes('', password);
      setCodes(result.codes ?? []);
      setStep('codes');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to generate recovery codes');
    } finally {
      setLoading(false);
    }
  };

  const copyAll = () => {
    navigator.clipboard.writeText(codes.join('\n'));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const download = () => {
    const blob = new Blob([codes.join('\n')], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'nvbes-recovery-codes.txt';
    a.click();
    URL.revokeObjectURL(url);
  };

  if (step === 'stepup') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <StepUpForm
          onSuccess={() => setStep('password')}
          onCancel={() => void navigate({ to: '/account/security' })}
          description="Pour générer des codes de récupération, veuillez confirmer votre identité."
        />
      </div>
    );
  }

  if (step === 'password') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <form onSubmit={handleGenerate} className="w-full max-w-md space-y-4">
          <h1 className="text-2xl font-bold">Codes de récupération</h1>
          <p className="text-sm text-muted-foreground">
            Confirmez votre mot de passe pour générer de nouveaux codes de récupération.
          </p>
          <Input
            type="password"
            placeholder="Mot de passe"
            value={password}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPassword(e.target.value)}
            required
          />
          {error && <p className="text-sm text-destructive">{error}</p>}
          <div className="flex gap-2">
            <Button
              type="button"
              variant="outline"
              className="flex-1"
              onClick={() => void navigate({ to: '/account/security' })}
            >
              Annuler
            </Button>
            <Button type="submit" className="flex-1" disabled={loading}>
              {loading ? 'Génération...' : 'Générer'}
            </Button>
          </div>
        </form>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md space-y-6">
        <h1 className="text-2xl font-bold">Vos codes de récupération</h1>

        <div className="rounded-lg bg-destructive/10 p-4 text-sm text-destructive">
          Conservez ces codes dans un endroit sûr. Ils ne seront plus affichés. Chaque code ne peut
          être utilisé qu'une seule fois.
        </div>

        <div className="grid grid-cols-2 gap-2">
          {codes.map((code, i) => (
            <code
              key={`${code}-${i}`}
              className="rounded bg-muted px-3 py-2 text-sm font-mono text-center"
            >
              {code}
            </code>
          ))}
        </div>

        <div className="flex gap-2">
          <Button variant="outline" className="flex-1" onClick={copyAll}>
            {copied ? 'Copié !' : 'Copier'}
          </Button>
          <Button variant="outline" className="flex-1" onClick={download}>
            Télécharger
          </Button>
        </div>

        <Button className="w-full" onClick={() => void navigate({ to: '/account/security' })}>
          Retour à la sécurité
        </Button>
      </div>
    </div>
  );
}
