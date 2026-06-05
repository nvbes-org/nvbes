import { commonStyles, THEME } from '../theme.js';

export const invitationTemplate = {
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
      children: ['heading', 'intro', 'button', 'ignore'],
    },
    heading: {
      type: 'Heading',
      props: {
        text: "You've been invited to join {{workspace_name}}",
        style: commonStyles.h1,
      },
      children: [],
    },
    intro: {
      type: 'Text',
      props: {
        text: '{{inviter_name}} has invited you to collaborate on **{{workspace_name}}** in nvbes.',
        style: commonStyles.text,
      },
      children: [],
    },
    button: {
      type: 'Button',
      props: {
        text: 'Accept Invitation',
        href: '{{invitation_link}}',
        style: commonStyles.button,
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
