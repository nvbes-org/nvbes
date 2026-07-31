export type WebauthnSetupKind = 'passkey' | 'security_key';

export const setupCopy: Record<
  WebauthnSetupKind,
  {
    title: string;
    labelPlaceholder: string;
    stepUpText: string;
    successText: string;
    timeoutText: string;
    buttonText: string;
    unsupportedText: string;
    browserText: string;
  }
> = {
  passkey: {
    title: 'Ajouter une passkey',
    labelPlaceholder: 'Ex: Touch ID du Mac',
    stepUpText: 'Pour ajouter une passkey, veuillez confirmer votre identité.',
    successText: 'Votre passkey est maintenant active.',
    timeoutText: 'L’enregistrement de la passkey a expiré. Réessayez.',
    buttonText: 'Ajouter la passkey',
    unsupportedText:
      'La biométrie n’est pas disponible dans ce navigateur. Ouvrez cette page dans Chrome, Arc, Safari ou Edge.',
    browserText:
      'Si aucune fenêtre Touch ID ne s’ouvre, quittez le navigateur intégré VS Code/Electron et ouvrez cette page dans Chrome, Arc, Safari ou Edge.',
  },
  security_key: {
    title: 'Ajouter une clé de sécurité indépendante',
    labelPlaceholder: 'Ex: YubiKey bleue',
    stepUpText: 'Pour enregistrer une clé de sécurité, veuillez confirmer votre identité.',
    successText: 'Votre clé de sécurité est maintenant active.',
    timeoutText: 'Security key registration timed out. Please try again and touch your key.',
    buttonText: 'Enregistrer la clé',
    unsupportedText:
      'Les clés de sécurité WebAuthn ne sont pas disponibles dans ce navigateur. Ouvrez cette page dans Chrome, Arc, Safari ou Edge.',
    browserText:
      'Insérez votre clé avant de continuer. Si aucune demande ne s’affiche, utilisez Chrome, Arc, Safari ou Edge hors du navigateur intégré VS Code/Electron.',
  },
};
