import type { AccountMe, AccountWorkspace } from '@nvbes/identity-client';
import { Separator } from '@/components/ui/separator';
import { WorkspaceRow } from './WorkspacesPage.row';

export function VirtualWorkspacesList({
  me,
  workspaces,
  totalSize,
  virtualItems,
  measureElement,
}: {
  me: AccountMe | null;
  workspaces: AccountWorkspace[];
  totalSize: number;
  virtualItems: import('@tanstack/react-virtual').VirtualItem[];
  measureElement: (element: Element | null) => void;
}) {
  return (
    <div
      style={{
        height: `${totalSize}px`,
        position: 'relative',
      }}
    >
      {virtualItems.map((virtualItem) => {
        if (virtualItem.index % 2 === 1) {
          return (
            <div
              key={`separator-${virtualItem.index}`}
              className="absolute left-0 right-0 px-6"
              style={{ transform: `translateY(${virtualItem.start}px)` }}
            >
              <Separator className="my-1" />
            </div>
          );
        }

        const workspace = workspaces[Math.floor(virtualItem.index / 2)];

        return (
          <div
            key={workspace.id}
            ref={measureElement}
            data-index={virtualItem.index}
            className="absolute left-0 right-0 px-6"
            style={{ transform: `translateY(${virtualItem.start}px)` }}
          >
            <WorkspaceRow
              workspace={workspace}
              isCurrent={workspace.id === me?.current_workspace_id}
            />
          </div>
        );
      })}
    </div>
  );
}
