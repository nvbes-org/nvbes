import { Key, KeyRound, LifeBuoy, Smartphone } from 'lucide-react';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { StepUpMethod } from './useStepUpForm';

export function StepUpMethodSelect({
  method,
  hasWebAuthn,
  hasTotp,
  hasRecovery,
  onMethodChange,
}: {
  method: StepUpMethod;
  hasWebAuthn: boolean;
  hasTotp: boolean;
  hasRecovery: boolean;
  onMethodChange: (value: StepUpMethod) => void;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor="stepup-method" className="text-xs">
        Méthode de vérification
      </Label>
      <Select value={method} onValueChange={(value) => onMethodChange(value as StepUpMethod)}>
        <SelectTrigger id="stepup-method" className="w-full text-xs">
          <SelectValue placeholder="Choisir une méthode" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectItem value="password" className="text-xs">
              <span className="flex items-center gap-2">
                <Key className="size-3.5" /> Mot de passe
              </span>
            </SelectItem>
            {hasWebAuthn && (
              <SelectItem value="webauthn" className="text-xs">
                <span className="flex items-center gap-2">
                  <KeyRound className="size-3.5" /> Passkey / Clé de sécurité
                </span>
              </SelectItem>
            )}
            {hasTotp && (
              <SelectItem value="totp" className="text-xs">
                <span className="flex items-center gap-2">
                  <Smartphone className="size-3.5" /> Code TOTP
                </span>
              </SelectItem>
            )}
            {hasRecovery && (
              <SelectItem value="recovery" className="text-xs">
                <span className="flex items-center gap-2">
                  <LifeBuoy className="size-3.5" /> Code de récupération
                </span>
              </SelectItem>
            )}
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>
  );
}
