import type { AccountEntry, AccountMe, AccountWorkspace } from '@nvbes/identity-client';
import { identityClient } from '@nvbes/identity-client';
import { useQueryClient } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef, useState, type FormEvent } from 'react';

import { accountQueryKeys } from '@/account.queries';
import { useAccountContext } from '@/hooks/useAccountContext';

export function useWorkspacesPage() {
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newWorkspaceName, setNewWorkspaceName] = useState('');
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const { me, workspaces, loading } = useAccountContext();
  const queryClient = useQueryClient();
  const listRef = useRef<HTMLDivElement>(null);

  const rowCount = workspaces.length > 0 ? workspaces.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 8,
  });

  const handleCreate = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setCreateError(null);

    if (!newWorkspaceName.trim()) {
      setCreateError('Le nom du workspace est requis.');
      return;
    }

    setCreating(true);
    try {
      const workspace = await identityClient.createWorkspace({ name: newWorkspaceName.trim() });
      queryClient.setQueryData<{
        me: AccountMe | null;
        workspaces: AccountWorkspace[];
        accounts: AccountEntry[];
      }>(accountQueryKeys.context, (current) =>
        current
          ? {
              ...current,
              workspaces: [...current.workspaces, workspace],
            }
          : current,
      );
      setNewWorkspaceName('');
      setShowCreateDialog(false);
    } catch {
      setCreateError('Impossible de creer le workspace. Veuillez reessayer.');
    } finally {
      setCreating(false);
    }
  };

  const handleOpenCreate = () => {
    setNewWorkspaceName('');
    setCreateError(null);
    setShowCreateDialog(true);
  };

  return {
    createError,
    creating,
    handleCreate,
    handleOpenCreate,
    listRef,
    loading,
    me,
    newWorkspaceName,
    setNewWorkspaceName,
    setShowCreateDialog,
    showCreateDialog,
    totalSize: virtualizer.getTotalSize(),
    virtualItems: virtualizer.getVirtualItems(),
    virtualizer,
    workspaces,
  };
}
