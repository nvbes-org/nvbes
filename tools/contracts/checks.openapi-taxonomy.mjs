export function checkOpenApiTaxonomy({ errors, manifest, readJson }) {
  const accountApi = manifest.apis.find((api) => api.name === 'account-service');
  if (
    accountApi?.surface !== 'account' ||
    accountApi?.document !== 'apps/account-service/openapi.json'
  ) {
    errors.push(
      'contracts/openapi/manifest.json: account-service must describe the account surface',
    );
  }

  const developerApi = manifest.apis.find((api) => api.name === 'developer-service');
  if (
    developerApi?.surface !== 'developer' ||
    developerApi?.document !== 'apps/developer-service/openapi.json'
  ) {
    errors.push(
      'contracts/openapi/manifest.json: developer-service must describe the developer surface',
    );
  }

  const cloudApi = manifest.apis.find((api) => api.name === 'cloud-public-v1');
  if (
    cloudApi?.surface !== 'cloud-public' ||
    cloudApi?.versionPrefix !== '/v1' ||
    cloudApi?.document !== 'docs/api/openapi/cloud-public-v1.openapi.json'
  ) {
    errors.push(
      'contracts/openapi/manifest.json: cloud-public-v1 must describe the Cloud public /v1 surface',
    );
  }

  const cloudSpec = cloudApi ? readJson(cloudApi.document) : undefined;
  if (!cloudSpec) return;

  const serverUrls = (cloudSpec.servers ?? []).map((server) => server.url).join('\n');
  if (cloudSpec.info?.title !== 'nvbes Cloud API') {
    errors.push(`${cloudApi.document}: info.title must use Cloud branding`);
  }
  const legacyDriveHost = ['drive', 'nvbes', 'fr'].join('.');
  if (!serverUrls.includes('https://cloud.nvbes.fr') || serverUrls.includes(legacyDriveHost)) {
    errors.push(`${cloudApi.document}: servers must use Cloud branding`);
  }
}
