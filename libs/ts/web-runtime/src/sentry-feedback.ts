export function createSentryFeedbackOptions(appLabel: string) {
  return {
    autoInject: true,
    showBranding: false,
    showEmail: false,
    showName: false,
    isEmailRequired: false,
    isNameRequired: false,
    enableScreenshot: false,
    colorScheme: 'system' as const,
    triggerLabel: 'Signaler un bug',
    triggerAriaLabel: 'Signaler un bug',
    cancelButtonLabel: 'Annuler',
    submitButtonLabel: 'Envoyer',
    formTitle: 'Signaler un bug',
    messageLabel: 'Description',
    messagePlaceholder: 'Decrivez le probleme, ce que vous faisiez, et le resultat attendu.',
    successMessageText: 'Merci, votre retour a ete transmis.',
    isRequiredLabel: '(obligatoire)',
    tags: {
      app: appLabel,
      feature: 'user-feedback',
    },
  };
}
