#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { selectAffectedApplications } from './affected-applications.core.mjs';

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const catalog = JSON.parse(
  readFileSync(resolve(workspaceRoot, 'tools/ci/deployable-applications.json'), 'utf8'),
);

function argument(name) {
  const index = process.argv.indexOf(`--${name}`);
  return index === -1 ? null : process.argv[index + 1];
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: workspaceRoot,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(' ')} failed: ${result.error?.message ?? result.stderr}`,
    );
  }
  return result.stdout.trim();
}

function validateCommit(value, name) {
  if (!/^[0-9a-f]{40}$/u.test(value ?? '')) {
    throw new Error(`--${name} must be a full Git commit SHA`);
  }
}

function allForMode(mode) {
  return {
    rust: mode === 'web' ? [] : catalog.rust,
    web: mode === 'containers' ? [] : catalog.web,
  };
}

function selectedProject(mode, project) {
  const selected = allForMode(mode);
  const rust = selected.rust.filter((entry) => entry.project === project);
  const web = selected.web.filter((entry) => entry.project === project);
  if (rust.length + web.length !== 1) {
    throw new Error(`Project ${project} is not deployable in ${mode} mode`);
  }
  return { rust, web };
}

function main() {
  const mode = argument('mode');
  const project = argument('project');
  if (project) {
    process.stdout.write(JSON.stringify(selectedProject(mode, project)));
    return;
  }

  const base = argument('base');
  const head = argument('head');
  validateCommit(base, 'base');
  validateCommit(head, 'head');
  if (/^0+$/u.test(base)) {
    process.stdout.write(JSON.stringify(allForMode(mode)));
    return;
  }

  const changedPaths = run('git', ['diff', '--name-only', '--diff-filter=ACMRD', base, head])
    .split('\n')
    .filter(Boolean);
  const nxAffected =
    mode === 'containers'
      ? []
      : JSON.parse(
          run('pnpm', [
            'exec',
            'nx',
            'show',
            'projects',
            '--affected',
            `--base=${base}`,
            `--head=${head}`,
            '--type=app',
            '--json',
          ]),
        );
  const cargoMetadata =
    mode === 'web'
      ? { packages: [], workspace_members: [], resolve: { nodes: [] } }
      : JSON.parse(run('cargo', ['metadata', '--locked', '--format-version', '1']));

  process.stdout.write(
    JSON.stringify(
      selectAffectedApplications({
        catalog,
        changedPaths,
        nxAffected,
        cargoMetadata,
        workspaceRoot,
        mode,
      }),
    ),
  );
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
