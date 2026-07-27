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
  id = 'register-region',
  loading,
  regions,
  value,
  onValueChange,
}: {
  detectedRegion: string | null;
  id?: string;
  reliability: 'high' | 'medium' | 'low' | 'none';
  loading: boolean;
  regions: SupportedRegion[];
  value: string;
  onValueChange: (value: string) => void;
}) {
  const selectValue = regionSelectValue(value, regions);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Label htmlFor={id}>
          Région <span className="text-destructive">*</span>
        </Label>
        {loading && (
          <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
            <Spinner className="size-3" />
            Détection en cours...
          </span>
        )}
      </div>
      <Select value={selectValue} onValueChange={onValueChange}>
        <SelectTrigger id={id} className="w-full">
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
      {!detectedRegion && (
        <p className="text-xs text-muted-foreground">
          Sélectionnez le pays où vos données seront stockées.
        </p>
      )}
    </div>
  );
}

function regionSelectValue(value: string, regions: SupportedRegion[]): string {
  return regions.some((entry) => entry.country_code === value) ? value : '';
}
