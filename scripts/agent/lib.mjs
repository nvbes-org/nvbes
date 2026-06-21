import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";

export const root = process.cwd();

export function readText(path) {
  return readFileSync(join(root, path), "utf8");
}

export function writeText(path, text) {
  writeFileSync(join(root, path), text);
}

export function exists(path) {
  try {
    statSync(join(root, path));
    return true;
  } catch {
    return false;
  }
}

export function walk(dir, predicate = () => true) {
  const absolute = join(root, dir);
  const found = [];

  function visit(path) {
    for (const entry of readdirSync(path, { withFileTypes: true })) {
      const next = join(path, entry.name);
      if (entry.isDirectory()) {
        visit(next);
        continue;
      }
      const rel = relative(root, next);
      if (predicate(rel)) {
        found.push(rel);
      }
    }
  }

  if (exists(dir)) {
    visit(absolute);
  }

  return found.sort();
}

export function parseFrontmatter(text) {
  if (!text.startsWith("---\n")) {
    return {};
  }

  const end = text.indexOf("\n---\n", 4);
  if (end === -1) {
    return {};
  }

  const lines = text.slice(4, end).split("\n");
  const data = {};
  let currentKey = "";

  for (const line of lines) {
    const match = /^([a-zA-Z0-9_-]+):\s*(.*)$/.exec(line);
    if (match) {
      currentKey = match[1];
      data[currentKey] = match[2].replace(/^["']|["']$/g, "");
      continue;
    }

    if (currentKey && /^\s+/.test(line)) {
      data[currentKey] = `${data[currentKey]}\n${line.trim()}`.trim();
    }
  }

  return data;
}

export function lineCount(path) {
  const text = readText(path);
  return text.length === 0 ? 0 : text.split("\n").length;
}

export function report(title, rows) {
  console.log(title);
  for (const row of rows) {
    console.log(row);
  }
}

export function ensureParent(path) {
  return dirname(join(root, path));
}
