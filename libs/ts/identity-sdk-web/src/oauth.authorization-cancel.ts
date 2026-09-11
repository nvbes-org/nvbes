import { IndexedDbDpopTransactionStore, type DpopTransactionStore } from './dpop.transaction-store';
import type { WebStorage } from './storage';

/** Client-origin action on explicit restart. Does not revoke a server grant or session. */
export async function discardAuthorizationRequest(config: {
  storage: WebStorage;
  dpopStore?: DpopTransactionStore;
}): Promise<void> {
  const transaction = config.storage.getTransaction();
  // Clear synchronously before asynchronous key deletion so a new request is never erased.
  config.storage.clearTransaction();
  if (transaction?.dpop)
    await (config.dpopStore ?? new IndexedDbDpopTransactionStore()).remove(transaction.dpop.keyId);
}
