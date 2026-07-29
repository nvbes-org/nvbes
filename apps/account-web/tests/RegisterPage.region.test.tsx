import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vite-plus/test';
import { RegionSelect } from '../src/pages/RegisterPage.region';
import {
  filterRegionSearchValue,
  regionSearchValue,
} from '../src/pages/RegisterPage.region.search';

const regions = [
  {
    country_code: 'FR',
    data_region: 'eu-west',
    legal_jurisdiction: 'FR',
    primary_timezone: 'Europe/Paris',
    timezones: ['Europe/Paris'],
    sub_region: 'Europe',
    display_name: 'France',
  },
];

describe('RegionSelect', () => {
  it('matches France from a partial country name', () => {
    expect(filterRegionSearchValue(regionSearchValue(regions[0]), 'Fra')).toBe(1);
    expect(filterRegionSearchValue(regionSearchValue(regions[0]), 'Belg')).toBe(0);
  });

  it('shows the placeholder when the controlled value is not supported', () => {
    const markup = renderToStaticMarkup(
      <RegionSelect
        detectedRegion="unsupported-region"
        reliability="none"
        loading={false}
        regions={regions}
        value="unsupported-region"
        onValueChange={() => undefined}
      />,
    );

    expect(markup).toContain('role="combobox"');
    expect(markup).toContain('Sélectionnez votre pays...');
    expect(markup).not.toContain('unsupported-region');
    expect(markup).not.toContain('<select');
  });

  it('shows the selected country in the searchable combobox', () => {
    const markup = renderToStaticMarkup(
      <RegionSelect
        detectedRegion="FR"
        reliability="high"
        loading={false}
        regions={regions}
        value="FR"
        onValueChange={() => undefined}
      />,
    );

    expect(markup).toContain('aria-expanded="false"');
    expect(markup).toContain('🇫🇷 France (Europe)');
  });
});
