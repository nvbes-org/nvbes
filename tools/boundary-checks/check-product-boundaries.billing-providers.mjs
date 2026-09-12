import { existsSync, readFileSync } from 'node:fs';

function checkBillingProviderNeutralSurface(errors) {
  const requiredProviderFiles = [
    ['libs/rust/billing/src/provider.rs', 'PROVIDER_CODES: &[&str] = &["stripe", "mollie", "cb"]'],
    [
      'apps/billing-service/migrations/0009_billing_provider_cb.sql',
      "ADD VALUE IF NOT EXISTS 'cb'",
    ],
  ];
  for (const [file, expected] of requiredProviderFiles) {
    if (!existsSync(file)) {
      errors.push(`${file}: required for provider-neutral Billing surface`);
      continue;
    }
    if (!readFileSync(file, 'utf8').includes(expected)) {
      errors.push(
        `${file}: Billing provider-neutral surface must include CB provider evidence ${expected}`,
      );
    }
  }
}

function checkBillingMultiPspContinuityEvidence(errors) {
  const providerSubscriptions = 'libs/rust/billing/src/db.provider_subscriptions.rs';
  if (!existsSync(providerSubscriptions)) {
    errors.push(`${providerSubscriptions}: required for multi-PSP subscription continuity`);
  } else {
    const content = readFileSync(providerSubscriptions, 'utf8');
    for (const expected of [
      'provider_subscription_fallback_eligible',
      'active_non_primary_provider_subscription_remains_fallback_eligible',
      'primary_provider_subscription_is_never_marked_as_fallback',
      'inactive_non_primary_provider_subscription_is_not_fallback_eligible',
      'demoted_primary',
      'fallback_eligible',
    ]) {
      if (!content.includes(expected)) {
        errors.push(`${providerSubscriptions}: missing multi-PSP continuity evidence ${expected}`);
      }
    }
  }

  const workspaceEffects = 'libs/rust/billing/src/stripe_webhook_workspace_effects.rs';
  if (!existsSync(workspaceEffects)) {
    errors.push(`${workspaceEffects}: required for primary-provider webhook workspace effects`);
  } else {
    const content = readFileSync(workspaceEffects, 'utf8');
    for (const expected of [
      'provider_subscription_applies_workspace_effects',
      'workspace_effects_apply_only_to_primary_provider_subscription',
    ]) {
      if (!content.includes(expected)) {
        errors.push(
          `${workspaceEffects}: missing primary-provider workspace effect evidence ${expected}`,
        );
      }
    }
  }
}

export function checkBillingProviderEvidence(errors) {
  checkBillingProviderNeutralSurface(errors);
  checkBillingMultiPspContinuityEvidence(errors);
}
