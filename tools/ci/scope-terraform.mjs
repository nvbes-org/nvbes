import { posix } from 'node:path';

// Reuse the project's HCL parser, without executing Terraform. Both revisions
// retain removed references; ambiguity selects every active stack.
export async function loadTerraformSources(files, readVersions) {
  const { parse } = await import('@cdktn/hcl2json');
  const sources = new Map();
  try {
    for (const file of files.filter(
      (path) => path.startsWith('infrastructure/') && /\.tf(?:\.json)?$/u.test(path),
    )) {
      if (file.endsWith('.tf.json')) return null;
      const directory = posix.dirname(file);
      for (const text of readVersions(file)) {
        const parsed = await parse(file, text);
        for (const bodies of Object.values(parsed.module ?? {})) {
          for (const { source } of bodies) {
            if (typeof source !== 'string' || source.includes('${') || source.includes('%{'))
              return null;
            if (source.startsWith('.'))
              sources.set(directory, [
                ...(sources.get(directory) ?? []),
                posix.normalize(posix.join(directory, source)),
              ]);
          }
        }
      }
    }
  } catch {
    return null;
  }
  return sources;
}

export function terraformProjects(paths, nodes, sources) {
  const stacks = Object.entries(nodes).filter(
    ([, { data }]) => data.targets?.['terraform:validate'],
  );
  const all = stacks.map(([name]) => name).sort();
  const changed = paths.filter(
    (path) => path.startsWith('infrastructure/') && !/\.(md|txt)$/u.test(path),
  );
  if (!changed.length) return [];
  if (!sources) return all;
  const owned = new Set();
  const selected = [];
  for (const [name, { data }] of stacks) {
    const reachable = new Set();
    const visit = (root) => {
      if (reachable.has(root)) return;
      reachable.add(root);
      for (const dependency of sources.get(root) ?? []) visit(dependency);
    };
    visit(data.root);
    let affected = false;
    for (const path of changed) {
      if ([...reachable].some((root) => path.startsWith(`${root}/`))) {
        owned.add(path);
        affected = true;
      }
    }
    if (affected) selected.push(name);
  }
  return owned.size === changed.length ? selected.sort() : all;
}
