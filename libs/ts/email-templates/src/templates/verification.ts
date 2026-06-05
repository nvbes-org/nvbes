import { commonStyles, THEME } from '../theme.js';

export const verificationTemplate = {
  root: 'html',
  elements: {
    html: {
      type: 'Html',
      props: { lang: 'en' },
      children: ['head', 'body'],
    },
    head: {
      type: 'Head',
      props: {},
      children: [],
    },
    body: {
      type: 'Body',
      props: { style: commonStyles.body },
      children: ['container'],
    },
    container: {
      type: 'Container',
      props: { style: commonStyles.container },
      children: ['header', 'content', 'footer'],
    },
    header: {
      type: 'Section',
      props: {
        style: {
          padding: '32px',
          textAlign: 'center',
          backgroundColor: THEME.colors.brand,
        },
      },
      children: ['logo'],
    },
    logo: {
      type: 'Heading',
      props: {
        text: 'NVBES',
        style: { color: '#ffffff', margin: 0, letterSpacing: '2px' },
      },
      children: [],
    },
    content: {
      type: 'Section',
      props: { style: { padding: '40px' } },
      children: ['heading', 'greeting', 'intro', 'button', 'expires', 'ignore'],
    },
    heading: {
      type: 'Heading',
      props: {
        text: 'Verify your email address',
        style: commonStyles.h1,
      },
      children: [],
    },
    greeting: {
      type: 'Text',
      props: {
        text: 'Hi {{user_name}},',
        style: { ...commonStyles.text, fontWeight: 'bold' },
      },
      children: [],
    },
    intro: {
      type: 'Text',
      props: {
        text: "Welcome to nvbes! We're excited to have you on board. Please click the button below to verify your email address and activate your account.",
        style: commonStyles.text,
      },
      children: [],
    },
    button: {
      type: 'Button',
      props: {
        text: 'Verify Email',
        href: '{{verification_link}}',
        style: commonStyles.button,
      },
      children: [],
    },
    expires: {
      type: 'Text',
      props: {
        text: 'This link will expire in {{expires_hours}} hours.',
        style: { ...commonStyles.text, fontSize: '14px', fontStyle: 'italic' },
      },
      children: [],
    },
    ignore: {
      type: 'Hr',
      props: {
        style: { ...commonStyles.text, borderColor: THEME.colors.border, margin: '32px 0' },
      },
      children: [],
    },
    footer: {
      type: 'Section',
      props: { style: { ...commonStyles.footer, paddingBottom: '40px' } },
      children: ['footer-text', 'address'],
    },
    'footer-text': {
      type: 'Text',
      props: {
        text: 'Sent with ❤️ by nvbes',
        style: { margin: 0 },
      },
      children: [],
    },
    address: {
      type: 'Text',
      props: {
        text: '123 Cloud Avenue, Tech City, Earth',
        style: { margin: '4px 0 0', fontSize: '10px' },
      },
      children: [],
    },
  },
};
