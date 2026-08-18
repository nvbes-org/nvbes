import { dirname, relative, sep } from 'node:path';

const rustGlobalPaths = new Set([
  'Cargo.lock',
  'Cargo.toml',
  'rust-toolchain.toml',
  'rustfmt.toml',
]);

const detectorPaths = [
  'tools/ci/affected-applications.core.mjs',
  'tools/ci/affected-applications.mjs',
  'tools/ci/deployable-applications.json',
];

function hasPath(changedPaths, candidate) {
  return changedPaths.some((path) => path === candidate || path.startsWith(`${candidate}/`));
}

function relativePackageRoot(manifestPath, workspaceRoot) {
  const root = relative(workspaceRoot, dirname(manifestPath)).split(sep).join('/');
  return root.startsWith('../') ? null : root;
}

function changedPackageIds(changedPaths, metadata, workspaceRoot) {
  const packageRoots = metadata.packages
    .map((pkg) => ({
      id: pkg.id,
      root: relativePackageRoot(pkg.manifest_path, workspaceRoot),
    }))
    .filter(({ root }) => root !== null)
    .sort((left, right) => right.root.length - left.root.length);

  return new Set(
    changedPaths.flatMap((path) => {
      const owner = packageRoots.find(({ root }) => path === root || path.startsWith(`${root}/`));
      return owner ? [owner.id] : [];
    }),
  );
}

function dependencyClosureContains(startId, targets, nodesById) {
  const pending = [startId];
  const visited = new Set();
  while (pending.length > 0) {
    const current = pending.pop();
    if (targets.has(current)) return true;
    if (visited.has(current)) continue;
    visited.add(current);
    for (const dependency of nodesById.get(current)?.deps ?? []) {
      pending.push(dependency.pkg);
    }
  }
  return false;
}

function cargoAffectedPackages(changedPaths, metadata, workspaceRoot) {
  if (
    changedPaths.some(
      (path) =>
        rustGlobalPaths.has(path) || path.startsWith('.cargo/') || path.startsWith('contracts/'),
    )
  ) {
    return new Set(metadata.workspace_members);
  }

  const changedIds = changedPackageIds(changedPaths, metadata, workspaceRoot);
  const nodesById = new Map((metadata.resolve?.nodes ?? []).map((node) => [node.id, node]));
  return new Set(
    metadata.workspace_members.filter((id) => dependencyClosureContains(id, changedIds, nodesById)),
  );
}

function packageIdsByName(metadata) {
  return new Map(metadata.packages.map((pkg) => [pkg.name, pkg.id]));
}

function forceAll(changedPaths, mode) {
  if (detectorPaths.some((path) => hasPath(changedPaths, path))) return true;
  if (mode === 'ci') return hasPath(changedPaths, '.github/workflows/ci-applications.yml');
  if (mode === 'containers') {
    return (
      hasPath(changedPaths, '.github/workflows/container-release.yml') ||
      hasPath(changedPaths, '.dockerignore') ||
      hasPath(changedPaths, 'tools/security/check-containers.mjs')
    );
  }
  return (
    hasPath(changedPaths, '.github/workflows/deploy-web.yml') ||
    hasPath(changedPaths, 'tools/web-build')
  );
}

export function selectAffectedApplications({
  catalog,
  changedPaths,
  nxAffected,
  cargoMetadata,
  workspaceRoot,
  mode,
}) {
  if (!new Set(['ci', 'containers', 'web']).has(mode)) {
    throw new Error(`Unsupported affected-applications mode: ${mode}`);
  }

  const nxProjects = new Set(nxAffected);
  const all = forceAll(changedPaths, mode);
  const affectedPackageIds =
    mode === 'web' ? new Set() : cargoAffectedPackages(changedPaths, cargoMetadata, workspaceRoot);
  const packageIds = mode === 'web' ? new Map() : packageIdsByName(cargoMetadata);

  const rust = catalog.rust.filter((application) => {
    if (mode === 'web') return false;
    if (all || nxProjects.has(application.project)) return true;
    if (affectedPackageIds.has(packageIds.get(application.package))) return true;
    return mode === 'containers' && hasPath(changedPaths, application.dockerfile);
  });

  const web = catalog.web.filter((application) => {
    if (mode === 'containers') return false;
    return all || nxProjects.has(application.project);
  });

  return { rust, web };
}
