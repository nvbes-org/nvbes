#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const ignoreRules = ['SC2129', 'SC2016', 'SC2155', 'SC2034'];
const ignoreArgs = ignoreRules.flatMap((rule) => ['-ignore', rule]);

function isBinaryAvailable(bin) {
  const res = spawnSync('which', [bin], { stdio: 'ignore' });
  return res.status === 0;
}

function runLint() {
  if (isBinaryAvailable('actionlint')) {
    const res = spawnSync('actionlint', ignoreArgs, { stdio: 'inherit' });
    process.exit(res.status ?? 1);
  } else if (isBinaryAvailable('docker')) {
    const dockerArgs = [
      'run',
      '--rm',
      '-v',
      `${process.cwd()}:/repo`,
      '-w',
      '/repo',
      'docker.io/rhysd/actionlint:latest@sha256:b1934ee5f1c509618f2508e6eb47ee0d3520686341fec936f3b79331f9315667',
      ...ignoreArgs,
    ];
    const res = spawnSync('docker', dockerArgs, { stdio: 'inherit' });
    process.exit(res.status ?? 1);
  } else {
    console.error('Neither actionlint nor docker is available in PATH.');
    process.exit(1);
  }
}

runLint();
