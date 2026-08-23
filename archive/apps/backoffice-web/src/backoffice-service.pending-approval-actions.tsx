import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Check, X } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import {
  approveKycProfile,
  approveMarketplaceApp,
  cancelRecoveryRequest,
  rejectKycProfile,
} from './backoffice-service.api';
import { strongConfirmationCode } from './backoffice-service.strong-confirmation';
import type { AdminCredentials, PendingApprovalItem } from './backoffice-service.types';

type ApprovalAction = 'approve_kyc' | 'approve_marketplace' | 'cancel_recovery' | 'reject_kyc';

export function PendingApprovalActions({
  credentials,
  item,
}: {
  credentials: AdminCredentials;
  item: PendingApprovalItem;
}) {
  const [selectedAction, setSelectedAction] = useState<ApprovalAction | null>(null);
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const queryClient = useQueryClient();
  const availableActions = actionsForItem(item);
  const expectedCode = selectedAction
    ? strongConfirmationCode(baseCode(selectedAction), item.target_id)
    : '';
  const isReady = reason.trim().length >= 12 && confirmCode.trim() === expectedCode;
  const mutation = useMutation<unknown>({
    mutationFn: async () => {
      if (!selectedAction) throw new Error('Action approval missing');
      return executeApprovalAction(credentials, item, selectedAction, {
        confirm_code: confirmCode,
        reason,
      });
    },
    onSuccess: async () => {
      setConfirmCode('');
      setReason('');
      setSelectedAction(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['pending-approvals'] }),
        queryClient.invalidateQueries({ queryKey: ['developer-center'] }),
        queryClient.invalidateQueries({ queryKey: ['billing-platform-center'] }),
        queryClient.invalidateQueries({ queryKey: ['identity-governance-center'] }),
        queryClient.invalidateQueries({ queryKey: ['internal-audit-events'] }),
      ]);
    },
  });

  if (availableActions.length === 0) {
    return <span className="text-muted-foreground text-xs">Review from source center</span>;
  }

  return (
    <div className="min-w-64 space-y-2">
      <div className="flex flex-wrap gap-2">
        {availableActions.map((action) => (
          <Button
            key={action}
            onClick={() => setSelectedAction(action)}
            size="sm"
            type="button"
            variant={selectedAction === action ? 'default' : 'outline'}
          >
            {actionIcon(action)}
            {actionLabel(action)}
          </Button>
        ))}
      </div>
      {selectedAction ? (
        <div className="space-y-2">
          <Input
            onChange={(event) => setConfirmCode(event.target.value)}
            placeholder={expectedCode}
            value={confirmCode}
          />
          <Textarea
            onChange={(event) => setReason(event.target.value)}
            placeholder="ticket APPROVAL-123 reviewed"
            value={reason}
          />
          <Button
            disabled={!isReady || mutation.isPending}
            onClick={() => mutation.mutate()}
            size="sm"
            type="button"
          >
            Execute approval
          </Button>
        </div>
      ) : null}
      {mutation.error ? <p className="text-destructive text-xs">{mutation.error.message}</p> : null}
    </div>
  );
}

function actionsForItem(item: PendingApprovalItem): ApprovalAction[] {
  if (item.target_type === 'marketplace_app') return ['approve_marketplace'];
  if (item.target_type === 'kyc_profile') return ['approve_kyc', 'reject_kyc'];
  if (item.target_type === 'recovery_request') return ['cancel_recovery'];
  return [];
}

function executeApprovalAction(
  credentials: AdminCredentials,
  item: PendingApprovalItem,
  action: ApprovalAction,
  body: { confirm_code: string; reason: string },
) {
  if (action === 'approve_marketplace')
    return approveMarketplaceApp(credentials, item.target_id, body);
  if (action === 'approve_kyc') return approveKycProfile(credentials, item.target_id, body);
  if (action === 'reject_kyc') return rejectKycProfile(credentials, item.target_id, body);
  return cancelRecoveryRequest(credentials, item.target_id, body);
}

function baseCode(action: ApprovalAction): string {
  if (action === 'approve_marketplace') return 'APPROVE MARKETPLACE APP';
  if (action === 'approve_kyc') return 'APPROVE KYC';
  if (action === 'reject_kyc') return 'REJECT KYC';
  return 'CANCEL RECOVERY';
}

function actionLabel(action: ApprovalAction): string {
  if (action === 'approve_marketplace') return 'Approve';
  if (action === 'approve_kyc') return 'Approve';
  if (action === 'reject_kyc') return 'Reject';
  return 'Cancel';
}

function actionIcon(action: ApprovalAction) {
  return action === 'reject_kyc' || action === 'cancel_recovery' ? (
    <X className="size-3.5" />
  ) : (
    <Check className="size-3.5" />
  );
}
