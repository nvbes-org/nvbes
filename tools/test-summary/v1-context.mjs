import { sha256, confinedRead } from './v1-bundle-verification.mjs';
import { validateManifest } from './v1-manifest.mjs';

export function loadV1(cwd) {
  const rootBytes = confinedRead(cwd, 'docs/testing/v1/manifest.json');
  const root = JSON.parse(rootBytes);
  const domainBytes = Object.entries(root.domains)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([, file]) => confinedRead(cwd, file));
  const domains = domainBytes.map((bytes) => JSON.parse(bytes));
  validateManifest(root, domains);
  const manifestDigest = sha256(JSON.stringify([rootBytes.toString(), ...domainBytes.map(String)]));
  return { root, domains, manifestDigest };
}
