# Enterprise Component Selection

This app follows the project component selection hierarchy from `AGENTS.md`.

- Registry components imported with CLI: `pnpm dlx shadcn@latest add card badge empty --yes -c apps/enterprise-web`.
- Generated registry components used: `Card`, `Empty`, and `Badge` from `apps/enterprise-web/src/components/ui`.
- Custom `EnterpriseLayout` and `EnterpriseSidebar` are justified because the project registry has primitives and product-specific sidebars, but no tenant-admin shell matching this app's route tree, grouping, and TanStack Router links.
- Custom `EnterpriseRouteState`, `OverviewPage`, and `ModuleShellPage` compose project registry primitives and Tailwind utilities for enterprise-specific content only.
- No external registry was needed for this slice.
