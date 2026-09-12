#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const ROOT = process.cwd();
const SOURCE_ROOTS = ['apps', 'libs/ts'];
const SKIPPED_DIRS = new Set([
  '.git',
  '.nx',
  'coverage',
  'dist',
  'node_modules',
  'playwright-report',
  'test-results',
]);
const SOURCE_EXTENSIONS = /\.(?:c|m)?[jt]sx?$/u;

const RULES = [
  {
    name: 'code-eval',
    pattern: /\beval\s*\(/u,
    message: 'Do not evaluate AJAX-controlled data as code.',
  },
  {
    name: 'function-constructor',
    pattern: /\bnew\s+Function\s*\(/u,
    message: 'Do not compile strings into executable JavaScript.',
  },
  {
    name: 'string-timer',
    pattern: /\b(?:setTimeout|setInterval)\s*\(\s*(['"`])/u,
    message: 'Do not pass strings to timer APIs.',
  },
  {
    name: 'inner-html-assignment',
    pattern: /\.innerHTML\s*=/u,
    message: 'Use React rendering or @nvbes/web-runtime VerifiedHtml for reviewed HTML.',
  },
  {
    name: 'insert-adjacent-html',
    pattern: /\.insertAdjacentHTML\s*\(/u,
    message: 'Use React rendering or @nvbes/web-runtime VerifiedHtml for reviewed HTML.',
  },
  {
    name: 'document-write',
    pattern: /\bdocument\.write(?:ln)?\s*\(/u,
    message: 'Do not write AJAX data directly into the document stream.',
  },
  {
    name: 'dangerous-react-html',
    pattern: /dangerouslySetInnerHTML/u,
    message: 'Use @nvbes/web-runtime VerifiedHtml with SafeHtml.',
    allowed: [/^libs\/ts\/web-runtime\/src\/safe-html\.tsx$/u],
  },
  {
    name: 'jsonp',
    pattern: /\bjsonp\b|[?&]callback=/iu,
    message: 'Do not use JSONP; use CORS with typed JSON APIs.',
  },
];

const failures = [];

for (const root of SOURCE_ROOTS) {
  for (const filePath of walk(join(ROOT, root))) {
    const relativePath = normalize(relative(ROOT, filePath));
    const text = readFileSync(filePath, 'utf8');
    const lines = text.split(/\r?\n/u);

    lines.forEach((line, index) => {
      if (line.includes('nvbes-ajax-security-ignore')) {
        return;
      }

      for (const rule of RULES) {
        if (!rule.pattern.test(line)) {
          continue;
        }
        if (rule.allowed?.some((allowed) => allowed.test(relativePath))) {
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
  console.error('AJAX security guardrails failed:');
  for (const failure of failures) {
    console.error(`- ${failure.path}:${failure.line} [${failure.rule}] ${failure.message}`);
  }
  process.exit(1);
}

console.log('AJAX security guardrails passed.');

function walk(directory) {
  const files = [];
  if (!existsSync(directory)) {
    return files;
  }

  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!SKIPPED_DIRS.has(entry.name)) {
        files.push(...walk(join(directory, entry.name)));
      }
      continue;
    }

    const path = join(directory, entry.name);
    if (statSync(path).isFile() && SOURCE_EXTENSIONS.test(path)) {
      files.push(path);
    }
  }

  return files;
}

function normalize(path) {
  return path.replaceAll('\\', '/');
}
