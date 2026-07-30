import { ChevronsUpDown } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Field, FieldError, FieldLabel } from '@/components/ui/field';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Spinner } from '@/components/ui/spinner';
import type { SupportedRegion } from '../identity.auth.api';
import { filterRegionSearchValue, regionSearchValue } from './RegisterPage.region.search';

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
  error,
  id = 'register-region',
  loading,
  regions,
  value,
  onValueChange,
}: {
  detectedRegion: string | null;
  error?: string;
  id?: string;
  reliability: 'high' | 'medium' | 'low' | 'none';
  loading: boolean;
  regions: SupportedRegion[];
  value: string;
  onValueChange: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const selectValue = regionSelectValue(value, regions);
  const selectedRegion = regions.find((entry) => entry.country_code === selectValue);

  return (
    <Field>
      <div className="flex items-center gap-2">
        <FieldLabel htmlFor={id}>
          Région <span className="text-destructive">*</span>
        </FieldLabel>
        {loading && (
          <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
            <Spinner className="size-3" />
            Détection en cours...
          </span>
        )}
      </div>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            id={id}
            type="button"
            variant="outline"
            role="combobox"
            aria-expanded={open}
            aria-busy={loading}
            aria-invalid={Boolean(error)}
            aria-describedby={error ? `${id}-error` : undefined}
            className="w-full justify-between font-normal"
          >
            <span className="truncate">
              {selectedRegion ? regionOptionLabel(selectedRegion) : 'Sélectionnez votre pays...'}
            </span>
            <ChevronsUpDown className="opacity-50" aria-hidden="true" />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-(--radix-popover-trigger-width) p-0">
          <Command filter={filterRegionSearchValue}>
            <CommandInput placeholder="Rechercher un pays..." />
            <CommandList className="mt-1">
              <CommandEmpty>Aucun pays trouvé.</CommandEmpty>
              {regions.map((entry) => (
                <CommandItem
                  key={entry.country_code}
                  value={regionSearchValue(entry)}
                  data-checked={entry.country_code === selectValue}
                  onSelect={() => {
                    onValueChange(entry.country_code);
                    setOpen(false);
                  }}
                >
                  <span className="truncate">{regionOptionLabel(entry)}</span>
                </CommandItem>
              ))}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
      <FieldError id={`${id}-error`}>{error}</FieldError>
      {!detectedRegion && (
        <p className="text-xs text-muted-foreground">
          Sélectionnez le pays où vos données seront stockées.
        </p>
      )}
    </Field>
  );
}

function regionSelectValue(value: string, regions: SupportedRegion[]): string {
  return regions.some((entry) => entry.country_code === value) ? value : '';
}

function regionOptionLabel(region: SupportedRegion): string {
  const flag = countryCodeToFlag(region.country_code);
  const name = region.display_name ?? region.country_code;
  const subRegion = region.sub_region ? ` (${region.sub_region})` : '';
  return `${flag ? `${flag} ` : ''}${name}${subRegion}`;
}
