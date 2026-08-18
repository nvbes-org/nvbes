import { Section } from 'react-email';
import { EmailButton } from './email-ui.button';
import { EmailNotice } from './email-ui.notice';
import { EmailShell } from './email-ui.shell';
import { emailTheme } from './email-ui.tokens';

export interface EmailActionTemplateProps {
  actionLabel: string;
  actionUrl: string;
  expiry: string;
  greeting: string;
  message: string;
  preview: string;
  timing: string;
  timezone: string;
  title: string;
  year: string;
}

export function EmailActionTemplate({
  actionLabel,
  actionUrl,
  expiry,
  greeting,
  message,
  preview,
  timing,
  timezone,
  title,
  year,
}: EmailActionTemplateProps) {
  return (
    <EmailShell preview={preview} title={title} year={year}>
      <Section style={bodyTextStyle}>{greeting}</Section>
      <Section style={bodyTextStyle}>{message}</Section>
      <Section style={{ paddingTop: 8 }}>
        <EmailButton href={actionUrl}>{actionLabel}</EmailButton>
      </Section>
      <EmailNotice>
        {timing} <strong style={{ color: emailTheme.body }}>{expiry}</strong>.
        <br />
        Time shown in {timezone}.
      </EmailNotice>
    </EmailShell>
  );
}

const bodyTextStyle = {
  color: emailTheme.body,
  fontSize: 16,
  lineHeight: '26px',
  msoLineHeightRule: 'exactly',
  paddingBottom: 16,
} as const;
