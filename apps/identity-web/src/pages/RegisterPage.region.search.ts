import type { SupportedRegion } from '../identity.auth.api';

export function regionSearchValue(region: SupportedRegion): string {
  return [region.display_name, region.country_code, region.sub_region, region.data_region]
    .filter((part): part is string => Boolean(part))
    .join(' ');
}

export function filterRegionSearchValue(value: string, search: string): number {
  return normalizeSearchValue(value).includes(normalizeSearchValue(search)) ? 1 : 0;
}

function normalizeSearchValue(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLocaleLowerCase('fr')
    .trim();
}
