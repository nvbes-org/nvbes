# Application archive

Applications parked while the platform is rebuilt service by service.

Archived sources stay versioned under `archive/apps/`, but are excluded from the
active Cargo, pnpm, and Nx workspaces. Restore an application by moving it back
to `apps/` and explicitly reconnecting its workspace configuration.

The email service remains active in `apps/email-worker`.
