import { MapPinIcon } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Spinner } from '@/components/ui/spinner';
import type { SupportedRegion } from '../identity.auth.api';

function countryCodeToFlag(code: string): string {
  const chars = code.toUpperCase().split('');
  if (chars.length !== 2) {
    return '';
  }

  return String.fromCodePoint(
    chars[0].charCodeAt(0) - 65 + 0x1f1e6,
    chars[1].charCodeAt(0) - 65 + 0x1f1e6,
  );
}

export function RegionSelect({
  detectedRegion,
  reliability,
  loading,
  regions,
  value,
  onValueChange,
}: {
  detectedRegion: string | null;
  reliability: 'high' | 'medium' | 'low' | 'none';
  loading: boolean;
  regions: SupportedRegion[];
  value: string;
  onValueChange: (value: string) => void;
}) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Label htmlFor="register-region">Région</Label>
        {loading && (
          <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
            <Spinner className="size-3" />
            Détection en cours...
          </span>
        )}
        {!loading && detectedRegion && (
          <Badge variant={reliability === 'none' ? 'secondary' : 'outline'}>
            <MapPinIcon data-icon="inline-start" />
            Détecté (
            {reliability === 'high' ? 'IP' : reliability === 'medium' ? 'Timezone' : 'Locale'}) :{' '}
            {detectedRegion}
          </Badge>
        )}
      </div>
      <Select value={value} onValueChange={onValueChange}>
        <SelectTrigger id="register-region" className="w-full">
          <SelectValue placeholder="Sélectionnez votre pays..." />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {regions.map((entry) => (
              <SelectItem key={entry.country_code} value={entry.country_code}>
                <span className="flex items-center gap-2">
                  <span className="text-base leading-none">
                    {countryCodeToFlag(entry.country_code)}
                  </span>
                  <span>{entry.display_name ?? entry.country_code}</span>
                  {entry.sub_region && (
                    <span className="text-xs text-muted-foreground">({entry.sub_region})</span>
                  )}
                </span>
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        {detectedRegion
          ? 'Région détectée automatiquement. Vous pouvez la modifier si nécessaire.'
          : 'Sélectionnez le pays où vos données seront stockées.'}
      </p>
    </div>
  );
}
