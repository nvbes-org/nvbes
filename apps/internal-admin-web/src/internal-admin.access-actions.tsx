import { useMutation, useQueryClient } from '@tanstack/react-query';
import { RotateCcw } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { suspendWorkspaceMembership } from './internal-admin.api';
import type { AccessActionResult, AdminCredentials, PrivilegedUser } from './internal-admin.types';

export function PrivilegedMembershipAction({
  credentials,
  disabled,
  row,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  row: PrivilegedUser;
}) {
  const queryClient = useQueryClient();
  const [isOpen, setIsOpen] = useState(false);
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<AccessActionResult | null>(null);
  const mutation = useMutation({
    mutationFn: () =>
      suspendWorkspaceMembership(credentials, row.workspace_id, row.principal_id, {
        confirm_code: confirmCode,
        reason,
      }),
    onSuccess: async (data) => {
      setResult(data);
      setIsOpen(false);
      setConfirmCode('');
      setReason('');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['access-center'] }),
        queryClient.invalidateQueries({ queryKey: ['workspace-detail', row.workspace_id] }),
        queryClient.invalidateQueries({ queryKey: ['user-detail', row.principal_id] }),
        queryClient.invalidateQueries({ queryKey: ['internal-audit-events'] }),
      ]);
    },
  });
  const isReasonReady = reason.trim().length >= 12;
  const expectedCode = 'SUSPEND ACCESS';
  const isConfirmationReady = confirmCode.trim() === expectedCode;

  return (
    <div className="mt-2">
      <Button
        disabled={disabled || mutation.isPending}
        onClick={() => {
          setResult(null);
          setIsOpen(true);
        }}
        size="sm"
        type="button"
        variant="destructive"
      >
        Suspend access
      </Button>
      {isOpen ? (
        <div className="bg-muted/30 mt-3 grid gap-3 rounded-md border p-3">
          <Textarea
            disabled={disabled || mutation.isPending}
            onChange={(event) => setReason(event.target.value)}
            placeholder="Motif audit, ticket, incident, approbation..."
            value={reason}
          />
          <Input
            className="font-mono text-xs"
            disabled={disabled || mutation.isPending}
            onChange={(event) => setConfirmCode(event.target.value)}
            placeholder={expectedCode}
            value={confirmCode}
          />
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-muted-foreground text-xs">
              Motif detaille et code exact {expectedCode} requis.
            </p>
            <div className="flex gap-2">
              <Button
                disabled={mutation.isPending}
                onClick={() => {
                  setIsOpen(false);
                  setConfirmCode('');
                  setReason('');
                }}
                size="sm"
                type="button"
                variant="outline"
              >
                <RotateCcw className="size-4" />
                Annuler
              </Button>
              <Button
                disabled={disabled || !isReasonReady || !isConfirmationReady || mutation.isPending}
                onClick={() => mutation.mutate()}
                size="sm"
                type="button"
                variant="destructive"
              >
                Confirmer
              </Button>
            </div>
          </div>
          {mutation.error ? (
            <p className="text-destructive text-xs">
              {mutation.error instanceof Error ? mutation.error.message : 'Action impossible'}
            </p>
          ) : null}
        </div>
      ) : null}
      {result ? (
        <p className="text-muted-foreground mt-2 text-xs">
          {result.audit_action}: {result.previous_status} vers {result.next_status}
        </p>
      ) : null}
    </div>
  );
}
