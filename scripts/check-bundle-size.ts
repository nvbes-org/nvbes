import fs from 'node:fs';
import path from 'node:path';

const MAX_JS_CHUNK_KB = 500;
const MAX_CSS_CHUNK_KB = 100;
const MAX_TOTAL_KB = 2000;

const rawDistDir = process.argv[2];
if (!rawDistDir) {
  console.error('Usage: tsx scripts/check-bundle-size.ts <dist-dir>');
  process.exit(1);
}

const cwd = process.cwd();
const distDir = path.resolve(cwd, rawDistDir);
const rel = path.relative(cwd, distDir);
if (rel.startsWith('..') || path.isAbsolute(rel)) {
  console.error('Invalid path: dist directory must stay within current directory');
  process.exit(1);
}

const manifestPath = path.resolve(distDir, '.vite', 'manifest.json');
const relManifest = path.relative(distDir, manifestPath);
if (relManifest.startsWith('..') || path.isAbsolute(relManifest)) {
  console.error('Invalid path: manifest must stay within dist directory');
  process.exit(1);
}
if (!fs.existsSync(manifestPath)) {
  console.warn(`No manifest at ${manifestPath}, skipping size check (build first).`);
  process.exit(0);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));

let totalBytes = 0;
let violations = 0;

for (const [name, entry] of Object.entries(manifest) as [
  string,
  { file: string; src?: string; size?: number },
][]) {
  const filePath = path.resolve(distDir, entry.file);
  const relFile = path.relative(distDir, filePath);
  if (relFile.startsWith('..') || path.isAbsolute(relFile)) continue;
  if (!fs.existsSync(filePath)) continue;

  const rawBytes = fs.statSync(filePath).size;
  totalBytes += rawBytes;

  const isCss = entry.file.endsWith('.css');
  const isJs = !isCss;
  const limitKB = isCss ? MAX_CSS_CHUNK_KB : MAX_JS_CHUNK_KB;
  const rawKB = Math.round(rawBytes / 1024);

  if (rawKB > limitKB) {
    console.error(
      `SIZE VIOLATION: ${entry.file} is ${rawKB} KB (limit: ${limitKB} KB for ${isCss ? 'CSS' : 'JS'})`,
    );
    violations++;
  }
}

const totalKB = Math.round(totalBytes / 1024);
if (totalKB > MAX_TOTAL_KB) {
  console.error(`TOTAL VIOLATION: total bundle is ${totalKB} KB (limit: ${MAX_TOTAL_KB} KB)`);
  violations++;
}

if (violations > 0) {
  console.error(`\n${violations} bundle size violation(s) found.`);
  process.exit(1);
}

console.log(
  `Bundle size OK: ${totalKB} KB total (max JS chunk: ${MAX_JS_CHUNK_KB} KB, max CSS: ${MAX_CSS_CHUNK_KB} KB)`,
);
