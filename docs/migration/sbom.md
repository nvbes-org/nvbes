# SBOM Coverage Manifest

## Status

The repository SBOM gate covers Node.js, Rust, Go, Python and container manifests with no pending ecosystem.

## Components

`pnpm check:sbom` produces a real CycloneDX JSON SBOM at
`.temp/security/nvbes.cdx.json`. The security workflow uploads that file as an
immutable CI artifact for every reviewed revision.

The artifact is generated from the complete repository filesystem by Trivy. It
contains the resolved package components and dependency relationships discovered
from the Node.js, Rust, Go, Python and container manifests.

## Regeneration

```bash
pnpm check:sbom
node tools/security/sbom.mjs --output=.temp/security/nvbes.cdx.json
```

Trivy is mandatory locally and pinned in CI. A missing generator, an invalid
CycloneDX document or an empty component inventory fails the gate.
