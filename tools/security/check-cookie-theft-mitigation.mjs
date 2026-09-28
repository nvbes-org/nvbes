#!/usr/bin/env node
import { runControlsRegistryCheck } from './controls-registry-check.mjs';

runControlsRegistryCheck({
  registryPath: 'docs/security/cookie-theft-mitigation-controls.json',
  requirementIdPattern: /^CTM_REQ_\d{3}$/u,
  controlIdPattern: /^CTM_CTRL_\d{3}$/u,
  packageScript: {
    scriptName: 'check:cookie-theft-mitigation',
    commandPath: 'tools/security/check-cookie-theft-mitigation.mjs',
    label: 'cookie theft',
  },
  failureTitle: 'Cookie Theft Mitigation controls failed',
  okMessage: 'Cookie Theft Mitigation controls OK',
  runDomainAssertions({ requireFileIncludes, requireNamedTests }) {
    function assertImplementation() {
      requireFileIncludes(
        'apps/identity-service/src/identity.auth.rs',
        ['identity_sessions', 'hash_token', 'expires_at', 'SESSION_TTL_HOURS'],
        'identity.auth.rs',
      );
      requireFileIncludes(
        'libs/rust/core/src/security.headers.rs',
        ['ACCEPT_CH_VALUE', 'CRITICAL_CH_VALUE', 'CLEAR_SITE_DATA_VALUE'],
        'security.headers.rs',
      );
      requireFileIncludes(
        'apps/identity-service/src/identity.http.rs',
        ['LoginRequest', 'LoginResponse', 'session_token'],
        'login routes',
      );
      requireFileIncludes(
        'apps/identity-service/src/identity.tokens.rs',
        ['AccessTokenClaims', 'TokenService', 'pub fn verify('],
        'session tokens',
      );
      requireFileIncludes(
        'libs/rust/core/src/http.error.rs',
        [
          'reauthentication_error_explicitly_requests_reauthentication',
          'requiring_reauthentication',
          'ErrorRecovery::Reauthenticate',
        ],
        'http.error.rs',
      );
      requireFileIncludes(
        'apps/identity-service/src/identity.auth.rs',
        ['identity.authenticated', 'audit(&mut tx, principal_id, "identity.authenticated")'],
        'identity.auth.rs',
      );
      requireNamedTests(
        [
          'opaque_tokens_have_fixed_entropy_and_are_hashed_at_rest',
          'access_token_is_rs256_audience_bound_and_short_lived',
          'session_token_prefers_bearer_then_cookie',
          'clear_site_data_header_clears_browser_state',
          'security_headers_deny_clickjacking_with_csp_and_legacy_header',
        ],
        'cookie-theft behavioral tests',
      );
    }

    function assertCookieAttributes() {
      requireFileIncludes(
        'libs/rust/core/src/security.headers.rs',
        [
          'CLEAR_SITE_DATA_VALUE',
          'insert_clear_site_data_header',
          'CSP_VALUE',
          "frame-ancestors 'none'",
        ],
        'security headers',
      );
    }

    assertImplementation();
    assertCookieAttributes();
  },
});
