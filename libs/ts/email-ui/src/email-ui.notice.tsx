import type { ReactNode } from 'react';
import { Section } from 'react-email';
import { emailTheme, outlookExactLineHeight } from './email-ui.tokens';

export function EmailNotice({ children }: { children: ReactNode }) {
  return (
    <Section style={{ paddingTop: 24 }}>
      <Section
        style={{
          backgroundColor: emailTheme.notice,
          border: `1px solid ${emailTheme.border}`,
          borderRadius: emailTheme.radius,
          color: '#52646d',
          fontSize: 14,
          lineHeight: '22px',
          ...outlookExactLineHeight,
          padding: '14px 16px',
        }}
      >
        {children}
      </Section>
    </Section>
  );
}
