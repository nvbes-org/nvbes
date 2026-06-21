import { parseFrontmatter, readText, report, walk } from "./lib.mjs";

const skillFiles = walk(".agents/skills", (path) => path.endsWith("/SKILL.md"));

const rows = skillFiles.map((path) => {
  const metadata = parseFrontmatter(readText(path));
  const name = metadata.name || path.split("/").at(-2);
  const description = (metadata.description || "").replace(/\s+/g, " ").trim();
  return { name, path, description };
});

report("# Agent Skills Index", [
  "",
  `Skills found: ${rows.length}`,
  "",
  "| Skill | Path | Description |",
  "|---|---|---|",
  ...rows.map((row) => {
    const description = row.description.replaceAll("|", "\\|");
    return `| ${row.name} | \`${row.path}\` | ${description} |`;
  }),
]);
