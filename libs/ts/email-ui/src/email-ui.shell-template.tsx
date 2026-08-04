import type { ReactNode } from 'react';
import { EmailShell } from './email-ui.shell';

export function EmailShellTemplate({
  children,
  preview,
  title,
  year,
}: {
  children: ReactNode;
  preview: string;
  title: string;
  year: string;
}) {
  return (
    <EmailShell preview={preview} title={title} year={year}>
      {children}
    </EmailShell>
  );
}
