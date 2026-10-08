#!/usr/bin/env node
import { runControlsRegistryCheck } from './controls-registry-check.mjs';

runControlsRegistryCheck({
  registryPath: 'docs/security/authorization-controls.json',
  requirementIdPattern: /^AUTHZ_REQ_\d{3}$/u,
  controlIdPattern: /^AUTHZ_CTRL_\d{3}$/u,
  packageScript: {
    scriptName: 'check:authorization',
    commandPath: 'tools/security/check-authorization-controls.mjs',
    label: 'authorization',
  },
  failureTitle: 'Authorization controls failed',
  okMessage: 'Authorization controls OK',
  runDomainAssertions({ requireNamedTests }) {
    requireNamedTests(
      [
        'admin_cannot_manage_billing_or_delete_workspace',
        'viewer_is_read_only',
        'member_can_only_modify_owned_objects',
        'sensitive_actions_require_step_up',
        'only_active_personal_owners_and_consistent_team_owners_are_allowed',
        'step_up_required_for_sensitive_mutations',
      ],
      'authorization behavioral tests',
    );
  },
});
