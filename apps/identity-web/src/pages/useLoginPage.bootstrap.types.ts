import type { AccountEntry } from '@nvbes/identity-client';
import type { DecoyLinkTracker } from '@nvbes/identity-sdk-web';
import type { LoginStep } from './LoginProgress';

export type UseLoginPageBootstrapOptions = {
  checkingAuth: boolean;
  locationSearchStr: string;
  hasOAuthRequest: boolean;
  setCheckingAuth: (value: boolean) => void;
  setConnectedAccounts: (value: AccountEntry[]) => void;
  setStep: (value: LoginStep) => void;
  setEmail: (value: string) => void;
  setPassword: (value: string) => void;
  setError: (value: string | null) => void;
  navigateToAccount: () => void;
  authorizeCurrentOAuth: () => Promise<void>;
};

export type LoginPageDecoyRef = React.MutableRefObject<DecoyLinkTracker | null>;
