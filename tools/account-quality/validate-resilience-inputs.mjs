import process from 'node:process';
import { profileOptions } from '../load-tests/account/profiles.js';

const profile = process.env.K6_PROFILE;
const suite = process.env.K6_SUITE;

if (!['account-api', 'account-session'].includes(suite)) {
  throw new Error('K6_SUITE must be account-api or account-session.');
}

if (profile === 'scalability') {
  if (!process.env.ACCOUNT_SCALABILITY_COMPARISON_FILE) {
    throw new Error(
      'Scalability is an evidence-comparison gate; ACCOUNT_SCALABILITY_COMPARISON_FILE is required.',
    );
  }
  process.stdout.write('Validated scalability comparison input.\n');
  process.exit(0);
}

if (!['soak', 'spike', 'stress', 'volume'].includes(profile)) {
  throw new Error('Resilience requires volume, spike, stress, soak or scalability.');
}
profileOptions(profile, {
  ...process.env,
  ACCOUNT_K6_DURATION:
    process.env.ACCOUNT_K6_DURATION === 'profile-default' ? '' : process.env.ACCOUNT_K6_DURATION,
});

process.stdout.write(`Validated ${profile} resilience inputs.\n`);
