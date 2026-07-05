import { useState } from 'react';

import { driveScopesPreset, type SecretResult } from './WorkspaceServiceAccounts.helpers';

export function useWorkspaceServiceAccountsPageForms() {
  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState('');
  const [createDescription, setCreateDescription] = useState('');
  const [createRole, setCreateRole] = useState('member');
  const [createError, setCreateError] = useState<string | null>(null);

  const [clientDialogOpen, setClientDialogOpen] = useState(false);
  const [clientName, setClientName] = useState('');
  const [clientScopes, setClientScopes] = useState(driveScopesPreset);
  const [clientAudiences, setClientAudiences] = useState('nvbes-cloud-service');
  const [clientResources, setClientResources] = useState('');
  const [clientRequiredAcr, setClientRequiredAcr] = useState('');
  const [clientAssertionRequired, setClientAssertionRequired] = useState(false);
  const [clientAssertionJwk, setClientAssertionJwk] = useState('');
  const [clientError, setClientError] = useState<string | null>(null);

  const [attachDialogOpen, setAttachDialogOpen] = useState(false);
  const [attachClientId, setAttachClientId] = useState('');
  const [attachError, setAttachError] = useState<string | null>(null);

  const [editError, setEditError] = useState<string | null>(null);

  const [updateName, setUpdateName] = useState('');
  const [updateDescription, setUpdateDescription] = useState('');
  const [updateRole, setUpdateRole] = useState('member');

  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [secretResult, setSecretResult] = useState<SecretResult | null>(null);
  const [secretDialogOpen, setSecretDialogOpen] = useState(false);

  const clearCreate = () => {
    setCreateName('');
    setCreateDescription('');
    setCreateRole('member');
    setCreateError(null);
  };

  const clearClient = () => {
    setClientName('');
    setClientScopes(driveScopesPreset);
    setClientAudiences('nvbes-cloud-service');
    setClientResources('');
    setClientRequiredAcr('');
    setClientAssertionRequired(false);
    setClientAssertionJwk('');
    setClientError(null);
  };

  const clearAttach = () => {
    setAttachClientId('');
    setAttachError(null);
  };

  return {
    createOpen,
    setCreateOpen,
    createName,
    setCreateName,
    createDescription,
    setCreateDescription,
    createRole,
    setCreateRole,
    createError,
    setCreateError,
    clientDialogOpen,
    setClientDialogOpen,
    clientName,
    setClientName,
    clientScopes,
    setClientScopes,
    clientAudiences,
    setClientAudiences,
    clientResources,
    setClientResources,
    clientRequiredAcr,
    setClientRequiredAcr,
    clientAssertionRequired,
    setClientAssertionRequired,
    clientAssertionJwk,
    setClientAssertionJwk,
    clientError,
    setClientError,
    attachDialogOpen,
    setAttachDialogOpen,
    attachClientId,
    setAttachClientId,
    attachError,
    setAttachError,
    editError,
    setEditError,
    updateName,
    setUpdateName,
    updateDescription,
    setUpdateDescription,
    updateRole,
    setUpdateRole,
    busyAction,
    setBusyAction,
    secretResult,
    setSecretResult,
    secretDialogOpen,
    setSecretDialogOpen,
    clearCreate,
    clearClient,
    clearAttach,
  };
}

export type WorkspaceServiceAccountsPageForms = ReturnType<
  typeof useWorkspaceServiceAccountsPageForms
>;
