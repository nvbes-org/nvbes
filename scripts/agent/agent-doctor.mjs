import { exists, lineCount, parseFrontmatter, readText, walk } from "./lib.mjs";

const failures = [];
const warnings = [];

function requireFile(path) {
  if (!exists(path)) {
    failures.push(`Missing required file: ${path}`);
  }
}

function requireText(path, pattern, message) {
  if (!exists(path)) {
    failures.push(`Missing required file: ${path}`);
    return;
  }

  if (!pattern.test(readText(path))) {
    failures.push(`${path}: ${message}`);
  }
}

for (const path of [
  "docs/agent/agent.contract.md",
  "docs/agent/agent.routing.md",
  "docs/agent/agent.memory.md",
  "docs/agent/agent.skills.md",
  "docs/agent/agent.codex.md",
  "docs/agent/agent.workflows.md",
  "docs/agent/agent.index.md",
  ".agents/registry.yaml",
  ".codex/instructions.md",
]) {
  requireFile(path);
}

for (const path of walk(".agents/profiles", (file) => file.endsWith(".md"))) {
  requireText(path, /^## Role$/m, "profile must include ## Role");
  requireText(path, /^## .*Validation|^## Definition Of Done/m, "profile must define validation or done criteria");
}

for (const path of walk(".agents/workflows", (file) => file.endsWith(".md"))) {
  requireText(path, /^## Trigger$/m, "workflow must include ## Trigger");
  requireText(path, /## Steps|## Rules/m, "workflow must include steps or rules");
}

for (const path of walk(".agents/memory", (file) => file.endsWith(".md"))) {
  const lines = lineCount(path);
  if (lines > 200) {
    warnings.push(`${path}: memory file has ${lines} lines; prefer under 200`);
  }
}

for (const path of walk(".agents/skills", (file) => file.endsWith("/SKILL.md"))) {
  const text = readText(path);
  const metadata = parseFrontmatter(text);
  const lines = lineCount(path);

  if (lines > 500) {
    warnings.push(`${path}: skill has ${lines} lines; prefer progressive disclosure`);
  }

  if (!metadata.description) {
    failures.push(`${path}: missing frontmatter description`);
  }
}

if (failures.length > 0) {
  console.error("Agent doctor failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
}

if (warnings.length > 0) {
  console.warn("Agent doctor warnings:");
  for (const warning of warnings) {
    console.warn(`- ${warning}`);
  }
}

if (failures.length === 0 && warnings.length === 0) {
  console.log("Agent doctor passed");
}

process.exitCode = failures.length > 0 ? 1 : 0;
