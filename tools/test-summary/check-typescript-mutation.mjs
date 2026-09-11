import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export function mutationScore(report) {
  assert(report?.files && typeof report.files === 'object', 'Missing Stryker files');
  let caught = 0;
  let missed = 0;
  let unviable = 0;
  const ids = new Set();
  for (const [file, result] of Object.entries(report.files)) {
    assert(Array.isArray(result.mutants), 'Missing Stryker mutants');
    for (const mutant of result.mutants) {
      const key = `${file}:${mutant.id}`;
      assert(!ids.has(key), 'Duplicate mutant');
      ids.add(key);
      switch (mutant.status) {
        case 'Killed':
          caught++;
          break;
        case 'Survived':
        case 'NoCoverage':
        case 'Timeout':
          missed++;
          break;
        case 'CompileError':
          unviable++;
          break;
        default:
          throw new Error(`Unsupported or incomplete mutant result: ${mutant.status}`);
      }
    }
  }
  assert(caught + missed > 0, 'No viable mutants measured');
  return { caught, missed, unviable, score: (100 * caught) / (caught + missed) };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const result = mutationScore(JSON.parse(readFileSync(process.argv[2], 'utf8')));
    console.log(JSON.stringify(result));
    if (result.score < 90) process.exitCode = 1;
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
