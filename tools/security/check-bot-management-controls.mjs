#!/usr/bin/env node
import { runControlsRegistryCheck } from './controls-registry-check.mjs';

runControlsRegistryCheck({
  registryPath: 'docs/security/bot-management-controls.json',
  requirementIdPattern: /^BOT_REQ_\d{3}$/u,
  controlIdPattern: /^BOT_CTRL_\d{3}$/u,
  packageScript: {
    scriptName: 'check:bot-management',
    commandPath: 'tools/security/check-bot-management-controls.mjs',
    label: 'bot management',
  },
  failureTitle: 'Bot Management controls failed',
  okMessage: 'Bot Management controls OK',
  runDomainAssertions({ requireNamedTests }) {
    requireNamedTests(
      [
        'rate_limiter_blocks_after_limit',
        'concurrent_requests_share_a_single_limit',
        'check_helpers_apply_multiple_rules',
        'step_up_required_for_sensitive_mutations',
        'request_signature_includes_method_path_and_body_hash',
        'validate_password_rejects_weak_passwords',
      ],
      'bot-management behavioral tests',
    );
  },
});
