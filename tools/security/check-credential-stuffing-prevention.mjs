#!/usr/bin/env node
import { runControlsRegistryCheck } from './controls-registry-check.mjs';

runControlsRegistryCheck({
  registryPath: 'docs/security/credential-stuffing-prevention-controls.json',
  requirementIdPattern: /^CSPREV_REQ_\d{3}$/u,
  controlIdPattern: /^CSPREV_CTRL_\d{3}$/u,
  packageScript: {
    scriptName: 'check:credential-stuffing-prevention',
    commandPath: 'tools/security/check-credential-stuffing-prevention.mjs',
    label: 'credential stuffing',
  },
  failureTitle: 'Credential Stuffing Prevention controls failed',
  okMessage: 'Credential Stuffing Prevention controls OK',
  runDomainAssertions({ requireFileIncludes }) {
    function assertImplementation() {
      requireFileIncludes(
        'libs/rust/core/src/limiter.rs',
        ['RateLimiter', 'RateLimitRule', 'check_rate_limit', 'max_hits', 'rate_limited'],
        'limiter.rs',
      );
    }

    function assertLayeredDefenses() {
      requireFileIncludes(
        'apps/identity-service/src/identity.auth.rs',
        ['dummy_verify_password', 'normalize_email', 'authentication failed'],
        'identity.auth.rs',
      );
      requireFileIncludes(
        'libs/rust/core/src/auth.helpers.password.rs',
        ['hash_password', 'verify_password', 'argon2'],
        'auth.helpers.password.rs',
      );
      requireFileIncludes(
        'libs/rust/core/src/auth.helpers.validation.rs',
        ['validate_password', 'MIN_PASSWORD_LENGTH', 'MAX_PASSWORD_LENGTH'],
        'auth.helpers.validation.rs',
      );
      requireFileIncludes(
        'libs/rust/trust-risk/src/trust_risk.assessment.rs',
        ['Assessment', 'instantaneous_signals', 'RiskSignal'],
        'trust_risk.assessment.rs',
      );
    }

    assertImplementation();
    assertLayeredDefenses();
  },
});
