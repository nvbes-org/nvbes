import type { RefObject } from 'react';
import type { VirtualItem } from '@tanstack/react-virtual';
import type { AccountMe, AccountWorkspace } from '@nvbes/identity-client';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { EmptyWorkspaceState } from './WorkspacesPage.empty';
import { VirtualWorkspacesList } from './WorkspacesPage.virtual';

export function WorkspacesList({
  me,
  workspaces,
  listRef,
  totalSize,
  virtualItems,
  measureElement,
  onOpenCreate,
}: {
  me: AccountMe | null;
  workspaces: AccountWorkspace[];
  listRef: RefObject<HTMLDivElement | null>;
  totalSize: number;
  virtualItems: VirtualItem[];
  measureElement: (element: Element | null) => void;
  onOpenCreate: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Mes workspaces</CardTitle>
        <CardDescription>
          {workspaces.length} workspace{workspaces.length !== 1 ? 's' : ''} disponible
          {workspaces.length !== 1 ? 's' : ''}.
        </CardDescription>
      </CardHeader>
      <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
        {workspaces.length === 0 ? (
          <EmptyWorkspaceState onOpenCreate={onOpenCreate} />
        ) : (
          <VirtualWorkspacesList
            me={me}
            workspaces={workspaces}
            totalSize={totalSize}
            virtualItems={virtualItems}
            measureElement={measureElement}
          />
        )}
      </CardContent>
    </Card>
  );
}
