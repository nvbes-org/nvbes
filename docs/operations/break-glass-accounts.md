# Break-Glass Accounts

Break-glass accounts are emergency tenant admin accounts used only when normal admin access is unavailable.

## Controls

- Only tenant owners can activate or revoke break-glass status.
- The target account must already have active owner or admin access.
- Every activation, revocation, admin elevation, and emergency use writes an `audit_events` record.
- A break-glass account must provide an emergency procedure reason before admin elevation is granted.
- The procedure reference is stored with the account and reused during emergency elevation.

## Activation

1. Open Enterprise > Users.
2. Select the owner or admin account.
3. In Emergency access, enter the incident or runbook reference.
4. Enter the audit reason.
5. Mark the account as break-glass.

## Use

1. Sign in as the break-glass account.
2. Start the sensitive admin action.
3. Complete step-up verification.
4. Enter the emergency procedure reason in the admin verification dialog.
5. Complete only the actions covered by the procedure.

## Revocation

1. Open Enterprise > Users.
2. Select the break-glass account.
3. Enter the audit reason.
4. Revoke emergency access.

## Audit Review

Review these event types after every emergency procedure:

- `enterprise.break_glass.activated`
- `enterprise.break_glass.used`
- `enterprise.break_glass.revoked`
- `enterprise.admin_elevation.granted`

The review should verify the actor, target account, procedure reference, stated reason, and action timing.
