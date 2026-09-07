import { appendFileSync } from 'node:fs';
import { gateFailures } from './ci-gate.core.mjs';

const needs = JSON.parse(process.env.NVBES_CI_NEEDS);
const failures = gateFailures(needs);
appendFileSync(
  process.env.GITHUB_STEP_SUMMARY,
  `## CI gate\n\n${Object.entries(needs)
    .map(([name, job]) => `- ${name}: ${job.result}`)
    .join('\n')}\n\n${failures.length ? failures.join('\n') : 'All required lanes succeeded.'}\n`,
);
if (failures.length) throw new Error(failures.join('; '));
