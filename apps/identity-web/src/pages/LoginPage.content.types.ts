import type { useLoginPage } from './useLoginPage';

export type LoginPageContentProps = ReturnType<typeof useLoginPage> & {
  transitionDirection: 'forward' | 'backward';
};
