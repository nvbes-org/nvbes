import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const ROOT = process.cwd();
const APP_SRC_ROOTS = readdirSync(join(ROOT, 'apps'))
  .filter((entry) => entry.endsWith('-web'))
  .map((entry) => join(ROOT, 'apps', entry, 'src'));

const RULES = [
  {
    name: 'native fetch',
    pattern: /(?<![\w.])fetch\s*\(/u,
    message: 'Use @nvbes/web-runtime verifiedFetch/verifiedFetchJson outside approved runtime files.',
    allowed: [
      /(^|\/)sw\.ts$/u,
      /\.sw\./u,
      /service-worker/u,
      /api-monitor\.ts$/u,
      /\.test\./u,
    ],
  },
  {
    name: 'native clipboard',
    pattern: /navigator\.clipboard/u,
    message: 'Use @nvbes/web-ui ClipboardButton or an approved runtime clipboard primitive.',
    allowed: [/\.test\./u],
  },
  {
    name: 'browser storage',
    pattern: /(?<![\w.])(?:window\.)?(?:localStorage|sessionStorage)\s*\./u,
    message: 'Use @nvbes/web-runtime safe-storage wrappers.',
    allowed: [/(^|\/)sw\.ts$/u, /\.sw\./u, /\.test\./u],
  },
  {
    name: 'dangerous html',
    pattern: /dangerouslySetInnerHTML/u,
    message: 'Use @nvbes/web-runtime VerifiedHtml with SafeHtml.',
    allowed: [/\.test\./u],
  },
];

const failures = [];

for (const srcRoot of APP_SRC_ROOTS) {
  if (!existsDirectory(srcRoot)) {
    continue;
  }

  for (const filePath of walk(srcRoot)) {
    if (!/\.(ts|tsx)$/u.test(filePath)) {
      continue;
    }

    const relativePath = normalize(relative(ROOT, filePath));
    const text = readFileSync(filePath, 'utf8');
    const lines = text.split(/\r?\n/u);
    lines.forEach((line, index) => {
      if (line.includes('nvbes-guardrail-ignore')) {
        return;
      }

      for (const rule of RULES) {
        if (!rule.pattern.test(line)) {
          continue;
        }
        if (rule.allowed.some((allowed) => allowed.test(relativePath))) {
          continue;
        }

        failures.push({
          line: index + 1,
          message: rule.message,
          path: relativePath,
          rule: rule.name,
        });
      }
    });
  }
}

if (failures.length > 0) {
  console.error('Web runtime guardrails failed:');
  for (const failure of failures) {
    console.error(
      `- ${failure.path}:${failure.line} [${failure.rule}] ${failure.message}`,
    );
  }
  process.exit(1);
}

console.log('Web runtime guardrails passed.');

function existsDirectory(path) {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

function walk(directory) {
  const files = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      files.push(...walk(path));
    } else {
      files.push(path);
    }
  }
  return files;
}

function normalize(path) {
  return path.replaceAll('\\', '/');
}
