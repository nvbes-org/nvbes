export function isDocumentation(path) {
  // Executable fixtures and security/product contracts under docs are not prose.
  return /\.(md|mdx|txt|rst)$/u.test(path) && !path.startsWith('.github/actions/');
}

export function classifyPaths(paths, fallback = false) {
  const docsOnly = !fallback && paths.length > 0 && paths.every(isDocumentation);
  const ciOnly =
    !fallback &&
    !docsOnly &&
    paths.length > 0 &&
    paths.every(
      (path) =>
        isDocumentation(path) || path.startsWith('.github/') || path.startsWith('tools/ci/'),
    );
  return { docsOnly, ciOnly, graphRequired: fallback || (!docsOnly && !ciOnly) };
}

export const qualityTargets = ['format:check', 'lint', 'typecheck', 'check', 'test', 'build'];

export function typeScriptTargets(names, nodes) {
  return Object.fromEntries(
    qualityTargets.map((target) => [
      target,
      names.filter((name) => {
        const data = nodes[name]?.data;
        return (
          (data?.root?.startsWith('libs/ts/') || data?.metadata?.ci?.runtime === 'typescript') &&
          Object.hasOwn(data.targets ?? {}, target) &&
          (target !== 'build' || data?.metadata?.ci?.buildRequired === true)
        );
      }),
    ]),
  );
}

export function hasUnknownPath(paths, nodes) {
  const roots = Object.values(nodes)
    .map(({ data }) => data.root)
    .filter((root) => root && root !== '.');
  const globals = new Set([
    'Cargo.toml',
    'Cargo.lock',
    'pnpm-lock.yaml',
    'package.json',
    'nx.json',
    'pnpm-workspace.yaml',
    'tsconfig.base.json',
    'vite.config.ts',
    'rust-toolchain.toml',
    'rustfmt.toml',
    '.dockerignore',
  ]);
  return paths.some(
    (path) =>
      !isDocumentation(path) &&
      !globals.has(path) &&
      !roots.some((root) => path.startsWith(`${root}/`)) &&
      ![
        '.github/',
        '.cargo/',
        'tools/',
        'scripts/',
        'contracts/',
        'infrastructure/',
        'libs/rust/',
        'vendor/',
      ].some((root) => path.startsWith(root)),
  );
}
