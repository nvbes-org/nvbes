export const THEME = {
  colors: {
    brand: '#6c5ce7', // Vibrant purple
    brandDark: '#5849c4',
    text: '#1a1a2e',
    textMuted: '#6b7280',
    bg: '#ffffff',
    bgMuted: '#f9fafb',
    border: '#e5e7eb',
  },
  spacing: {
    container: '600px',
    padding: '40px',
  },
  fonts: {
    sans: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji"',
  },
};

export const commonStyles = {
  container: {
    maxWidth: THEME.spacing.container,
    margin: '0 auto',
    backgroundColor: THEME.colors.bg,
    borderRadius: '12px',
    overflow: 'hidden',
    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
    border: `1px solid ${THEME.colors.border}`,
  },
  body: {
    backgroundColor: THEME.colors.bgMuted,
    fontFamily: THEME.fonts.sans,
    padding: '40px 0',
  },
  button: {
    backgroundColor: THEME.colors.brand,
    borderRadius: '8px',
    color: '#fff',
    fontSize: '16px',
    fontWeight: 'bold',
    textDecoration: 'none',
    textAlign: 'center' as const,
    display: 'block',
    padding: '12px 24px',
    margin: '24px 0',
  },
  h1: {
    color: THEME.colors.text,
    fontSize: '24px',
    fontWeight: 'bold',
    margin: '0 0 16px',
    textAlign: 'center' as const,
  },
  text: {
    color: THEME.colors.textMuted,
    fontSize: '16px',
    lineHeight: '24px',
    margin: '16px 0',
  },
  footer: {
    color: THEME.colors.textMuted,
    fontSize: '12px',
    textAlign: 'center' as const,
    marginTop: '32px',
    padding: '0 40px',
  },
};
