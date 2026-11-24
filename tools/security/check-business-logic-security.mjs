#!/usr/bin/env node
import { runControlsRegistryCheck } from './controls-registry-check.mjs';

runControlsRegistryCheck({
  registryPath: 'docs/security/business-logic-security-controls.json',
  requirementIdPattern: /^BLS_REQ_\d{3}$/u,
  controlIdPattern: /^BLS_CTRL_\d{3}$/u,
  packageScript: {
    scriptName: 'check:business-logic-security',
    commandPath: 'tools/security/check-business-logic-security.mjs',
    label: 'business logic security',
  },
  failureTitle: 'Business Logic Security controls failed',
  okMessage: 'Business Logic Security controls OK',
  runDomainAssertions({ requireNamedTests }) {
    requireNamedTests(
      [
        'workspace_membership_roles_cover_expected_permission_boundaries',
        'validate_password_accepts_strong_password',
        'validate_password_rejects_weak_passwords',
      ],
      'business-logic behavioral tests',
    );
  },
});
