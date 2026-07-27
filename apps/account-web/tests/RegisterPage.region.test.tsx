import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vite-plus/test';
import { RegionSelect } from '../src/pages/RegisterPage.region';

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

    expect(markup).toContain('Sélectionnez votre pays...');
    expect(markup).not.toContain('unsupported-region');
  });
});
