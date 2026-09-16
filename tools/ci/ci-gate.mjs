import { appendFileSync } from 'node:fs';
import { gateFailures } from './ci-gate.core.mjs';

const needs = JSON.parse(process.env.NVBES_CI_NEEDS);
const failures = gateFailures(needs);
let summary = `## 🚪 CI Gate\n\n| Lane / Job | Result | Status |\n| :--- | :--- | :---: |\n`;
for (const [name, job] of Object.entries(needs)) {
  const icon = job.result === 'success' ? '✅' : job.result === 'skipped' ? '⏭️' : '❌';
  summary += `| \`${name}\` | \`${job.result}\` | ${icon} |\n`;
}
summary += `\n${failures.length ? `> [!CAUTION]\n> **Failures:**\n> - ${failures.join('\n> - ')}` : '> [!NOTE]\n> **All required CI lanes succeeded.**'}\n`;

appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
if (failures.length) throw new Error(failures.join('; '));
