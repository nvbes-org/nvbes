#!/usr/bin/env node
import { checkBillingMigrationBoundaries } from './check-product-boundaries.billing-migrations.mjs';
import { checkRustPackageBoundaries } from './check-product-boundaries.rust-packages.mjs';

const errors = [];

checkRustPackageBoundaries(errors);
checkBillingMigrationBoundaries(errors);

if (errors.length > 0) {
  console.error('Product boundary violations:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Product boundaries: ok');
