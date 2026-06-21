import { readText, writeText } from "./lib.mjs";

const write = process.argv.includes("--write");

const sources = [
  "docs/agent/agent.contract.md",
  "docs/agent/agent.routing.md",
  "docs/agent/agent.memory.md",
  "docs/agent/agent.skills.md",
  "docs/agent/agent.workflows.md",
  "docs/agent/agent.codex.md",
];

const generated = [
  "# nvbes Agent Instructions",
  "",
  "This file is generated from `docs/agent/*` by `pnpm agent:sync -- --write`.",
  "Edit canonical files instead of editing this file directly.",
  "",
  ...sources.flatMap((path) => [`<!-- source: ${path} -->`, readText(path), ""]),
].join("\n");

const target = ".codex/instructions.md";

if (write) {
  writeText(target, generated);
  console.log(`Updated ${target}`);
} else {
  const current = readText(target);
  if (current !== generated) {
    console.error(`${target} is not synchronized. Run pnpm agent:sync -- --write`);
    process.exitCode = 1;
  } else {
    console.log(`${target} is synchronized`);
  }
}
