import { parseFrontmatter, readText, report, walk } from './lib.mjs';

const files = walk('docs/research', (path) => path.endsWith('.md'));

const rows = files.map((path) => {
  const text = readText(path);
  const metadata = parseFrontmatter(text);
  const titleLine = text.split('\n').find((l) => l.startsWith('# '));
  const title = titleLine ? titleLine.replace(/^#[ \t]+/, '').trim() : path;
  return {
    title,
    path,
    topic: metadata.topic || '',
    status: metadata.status || '',
    reviewed: metadata.last_reviewed || '',
  };
});

report('# Research Index', [
  '',
  `Research files found: ${rows.length}`,
  '',
  '| Title | Topic | Status | Last reviewed | Path |',
  '|---|---|---|---|---|',
  ...rows.map((row) => {
    return `| ${row.title} | ${row.topic} | ${row.status} | ${row.reviewed} | \`${row.path}\` |`;
  }),
]);
