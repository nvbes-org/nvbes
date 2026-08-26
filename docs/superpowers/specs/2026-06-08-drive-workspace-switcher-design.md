# Drive Workspace Switcher Design

> **Status: future Cloud/Drive product design, outside active V1.** See the
> [active direction](../../product/nvbes-product-strategy.md).

Date: 2026-06-08

## Goal

Ajouter un sélecteur de workspace dans le header de cloud-web, permettant de changer de workspace (et implicitement de tenant) sans se déconnecter.

## Contexte

- `DriveMeResponse` contient déjà `workspaces: DriveWorkspaceView[]` et `current_workspace_id`
- `switchDriveWorkspace()` existe dans `drive.workspace.switch.ts` mais n'est jamais appelé
- L'endpoint `POST /auth/workspaces/{workspaceId}/switch` existe dans account-service
- L'API Account met à jour la session Redis, le Cloud API la lit partagée

## UI

Le nom du workspace dans `DriveCommandHeader` devient cliquable. Au click, un dropdown liste tous les workspaces accessibles :

```
┌─────────────────────────────────────────────┐
│  ▼ Mon Workspace              [search]  [⏤] │
│  Fichiers                                    │
└─────────────────────────────────────────────┘
         ↓
┌─────────────────────┐
│ ✓ Mon Workspace     │  ← actif
│   Équipe Commercial │
│   Client X          │
│                     │
│   [+ Créer]         │  lien vers /account/workspaces
└─────────────────────┘
```

## Flux

1. User clique sur le workspace actif → dropdown avec la liste
2. User sélectionne un workspace → appelle `POST /auth/workspaces/{id}/switch`
3. L'API identity met à jour la session Redis (workspace_id, tenant_id, etc.)
4. Invalide la query TanStack `me` → re-fetch avec le nouveau contexte
5. `DriveShell` re-render avec le workspace actif mis à jour

## Components

### Nouveau : `DriveWorkspaceSwitcher`
- Props: `accessToken`, `workspaceName`, `workspaceId`, `workspaces`
- Dropdown custom (pas de dépendance shadcn supplémentaire)
- Affiche la liste des workspaces, avec checkmark sur l'actif
- Au clic, appelle `switchDriveWorkspace()` puis invalide la query

### Modifié : `DriveCommandHeader`
- `workspaceName: string` → `workspaceName` + `workspaceSwitcher: ReactNode` (optionnel)
- Affiche le switcher à la place du simple texte

### Modifié : `DriveShell`
- Passe les props nécessaires à `DriveCommandHeader`

## Fichiers

- `src/DriveWorkspaceSwitcher.tsx` (nouveau)
- `src/DriveCommandHeader.tsx` (modifié)
- `src/DriveShell.tsx` (modifié)
