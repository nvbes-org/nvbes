import { hasUnknownPath, typeScriptTargets } from './scope-paths.mjs';
import { containerProjects, databaseProjects, isRustWorkspaceAffected } from './scope-runtimes.mjs';
import { terraformProjects } from './scope-terraform.mjs';

export const lanes = ['contracts', 'typescript', 'rust', 'database', 'terraform', 'containers'];

export function createPlan(
  preflight,
  {
    nodes = {},
    affected = [],
    terraformSources,
    read = () => '',
    graphFallback = false,
    globalConfigurationChanged = false,
    rolloutMode = 'shadow',
  } = {},
) {
  const { paths, docsOnly, ciOnly } = preflight;
  const names = Object.keys(nodes).sort();
  const fallback =
    preflight.fallback || graphFallback || (!docsOnly && !ciOnly && hasUnknownPath(paths, nodes));
  const allWith = (target) => names.filter((name) => nodes[name].data.targets?.[target]);
  const fullTypeScriptConfig = paths.some((path) =>
    ['pnpm-workspace.yaml', 'tsconfig.base.json', 'vite.config.ts'].includes(path),
  );
  const full = fallback || paths.includes('nx.json') || globalConfigurationChanged;
  const targets = typeScriptTargets(full || fullTypeScriptConfig ? names : affected, nodes);
  const databases = full ? allWith('test:database') : databaseProjects(paths, nodes, read);
  const terraform = full
    ? allWith('terraform:validate')
    : terraformProjects(paths, nodes, terraformSources);
  const containers = full
    ? allWith('test:contract').filter((name) => nodes[name].data.root.startsWith('apps/'))
    : containerProjects(paths, nodes, read);
  const rust = full || isRustWorkspaceAffected(paths);
  const workspaceRust =
    full ||
    paths.some(
      (path) =>
        ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'rustfmt.toml'].includes(path) ||
        ['.cargo/', 'vendor/', 'contracts/protobuf/'].some((prefix) => path.startsWith(prefix)),
    );
  const candidate = {
    contracts: !docsOnly,
    typescript: Object.values(targets).some((projects) => projects.length),
    rust,
    database: databases.length > 0,
    terraform: terraform.length > 0,
    containers: containers.length > 0,
  };
  const fastPath =
    docsOnly || ciOnly || (paths.length > 0 && paths.every((path) => path.endsWith('/Dockerfile')));
  const shadow = rolloutMode === 'shadow' && !fastPath && !full;
  const baseline = {
    targets: typeScriptTargets(names, nodes),
    databases: allWith('test:database'),
    terraform: allWith('terraform:validate'),
    containers: allWith('test:contract').filter((name) =>
      nodes[name].data.root.startsWith('apps/'),
    ),
  };
  return {
    version: 1,
    base: preflight.base,
    head: preflight.head,
    docsOnly,
    ciOnly,
    fallbackFull: fallback,
    reason: graphFallback ? 'Nx affected failed; all projects selected' : preflight.reason,
    targets,
    databases,
    terraform,
    containers,
    candidate,
    shadow,
    baseline,
    databaseExecution: shadow && candidate.database ? baseline.databases : databases,
    rustMode: rust
      ? rolloutMode === 'affected' && !workspaceRust
        ? 'scoped'
        : 'workspace'
      : 'none',
    paths,
    plannedAt: Date.now(),
  };
}
