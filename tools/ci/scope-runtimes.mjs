import { isDatabaseScopeAffected, isRustWorkspaceAffected } from './nx-cache-manager.core.mjs';

export function databaseProjects(paths, nodes, read = () => '') {
  const projects = Object.entries(nodes).filter(([, { data }]) => data.targets?.['test:database']);
  const persistence = (path) =>
    /(?:sqlx\b|PgPool\b|PgConnection\b|(?:database|persistence|repository)::)/u.test(read(path));
  const shared = paths.some(
    (path) =>
      path === 'Cargo.lock' ||
      path === 'Cargo.toml' ||
      path.startsWith('scripts/lib/') ||
      path.startsWith('.sqlx/') ||
      (path.startsWith('libs/rust/') &&
        (/(?:db|database|persistence|repository|migration|sql)/u.test(path) ||
          (path.endsWith('.rs') && persistence(path)))),
  );
  return projects
    .filter(([name, { data }]) => {
      const scope = name === 'email-worker' ? 'email' : name.replace(/-service$/u, '');
      return (
        shared ||
        isDatabaseScopeAffected(paths, scope) ||
        paths.some(
          (path) =>
            path.startsWith(`${data.root}/`) &&
            (/(?:db[./]|database|persistence|repository|migration|\.sql|Cargo\.toml|tests\/)/u.test(
              path,
            ) ||
              (path.endsWith('.rs') && persistence(path))),
        )
      );
    })
    .map(([name]) => name)
    .sort((a, b) => a.localeCompare(b));
}

export function containerProjects(paths, nodes, read) {
  return Object.entries(nodes)
    .filter(([, { data }]) => {
      if (!data.root.startsWith('apps/') || !data.targets?.['test:contract']) return false;
      if (
        paths.some(
          (path) => path === `${data.root}/Dockerfile` || path.startsWith(`${data.root}/tests/`),
        )
      )
        return true;
      const dockerfile = read(`${data.root}/Dockerfile`);
      if (!dockerfile) return true;
      const copies = [...dockerfile.matchAll(/^(?:COPY|ADD)\s+([^\n]+)/gmu)].map(
        (match) => match[1],
      );
      const local = copies.filter((copy) => !copy.includes('--from='));
      // JSON form, globbing or interpolation is ambiguous: keep every source.
      const ambiguous = local.some((copy) => /[[\]$*?\\]/u.test(copy));
      const roots = local.flatMap((copy) =>
        copy
          .split(/\s+/u)
          .filter((word) => !word.startsWith('--'))
          .slice(0, -1),
      );
      return paths.some(
        (path) =>
          !path.endsWith('/Dockerfile') &&
          (path === '.dockerignore' ||
            path.startsWith('.github/workflows/') ||
            ambiguous ||
            roots.some(
              (root) =>
                root === '.' || path === root || path.startsWith(`${root.replace(/\/$/u, '')}/`),
            )),
      );
    })
    .map(([name]) => name)
    .sort((a, b) => a.localeCompare(b));
}

export { isRustWorkspaceAffected };
