import type { ReactNode } from 'react';
import { Body, Container, Head, Heading, Html, Preview, Section } from 'react-email';
import { emailTheme, outlookExactLineHeight } from './email-ui.tokens';

export function EmailShell({
  children,
  preview,
  product = 'Account',
  title,
  year,
}: {
  children: ReactNode;
  preview: string;
  product?: string;
  title: string;
  year: string;
}) {
  return (
    <Html lang="en">
      <Head />
      <Preview>{preview}</Preview>
      <Body style={{ backgroundColor: emailTheme.background, margin: 0 }}>
        <Container
          style={{
            color: emailTheme.body,
            fontFamily: emailTheme.fontFamily,
            maxWidth: 632,
            padding: '40px 16px 24px',
          }}
        >
          <Section
            style={{
              backgroundColor: emailTheme.card,
              border: `1px solid ${emailTheme.border}`,
              borderRadius: emailTheme.cardRadius,
              padding: '28px 40px 40px',
            }}
          >
            <Section
              style={{
                color: emailTheme.primary,
                fontSize: 15,
                fontWeight: 700,
                letterSpacing: '-0.01em',
                lineHeight: '24px',
                ...outlookExactLineHeight,
                paddingBottom: 28,
              }}
            >
              nvbes <span style={{ color: emailTheme.muted, fontWeight: 500 }}>{product}</span>
            </Section>
            <Heading
              as="h1"
              style={{
                color: emailTheme.foreground,
                fontSize: 30,
                fontWeight: 700,
                letterSpacing: '-0.035em',
                lineHeight: '36px',
                ...outlookExactLineHeight,
                margin: '0 0 24px',
              }}
            >
              {title}
            </Heading>
            {children}
          </Section>
          <Section
            style={{
              color: emailTheme.muted,
              padding: '18px 8px 0',
            }}
          >
            <Section style={footerTextStyle}>
              This transactional email was sent because an action was requested for your nvbes
              account. If you did not request it, you can safely ignore this message.
            </Section>
            <Section style={footerTextStyle}>© {year} nvbes · Identity &amp; Account</Section>
            <Section style={footerTextStyle}>
              <LegalLink href="https://nvbes.fr/legal/site-legal-notice">Legal notice</LegalLink>
              {' · '}
              <LegalLink href="https://nvbes.fr/legal/privacy-policy">Privacy policy</LegalLink>
              {' · '}
              <LegalLink href="https://nvbes.fr/legal/terms-of-service">Terms of service</LegalLink>
              {' · '}
              <LegalLink href="https://nvbes.fr/legal/data-processing-agreement">
                Data processing agreement
              </LegalLink>
            </Section>
          </Section>
        </Container>
      </Body>
    </Html>
  );
}

function LegalLink({ children, href }: { children: ReactNode; href: string }) {
  return (
    <a href={href} style={{ color: emailTheme.muted }} target="_blank">
      {children}
    </a>
  );
}

const footerTextStyle = {
  color: emailTheme.muted,
  fontSize: 12,
  lineHeight: '18px',
  msoLineHeightRule: 'exactly',
  paddingBottom: 8,
} as const;
