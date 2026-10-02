# Clé non résidente dans Identity Web

Une clé non découvrable ne sélectionne pas le compte au démarrage du login
passkey. Le parcours actif utilise d'abord email et mot de passe, puis le
step-up WebAuthn lié à la session. Le serveur fournit les credential IDs de ce
compte dans `allowCredentials` et exige la vérification utilisateur. Le
navigateur ne recherche pas les clés d'un email anonyme.

Le profil client `recent_webauthn` refuse le consentement tant que cette preuve
n'est pas satisfaite. L'enregistrement du premier facteur, puis sa confirmation,
utilisent les contrôleurs actifs et l'habillage Identity archivé. L'écran indique
comment continuer si la clé ne propose pas le compte et nomme explicitement les
clés de sécurité au step-up. Le bouton revient à la ligne sur mobile.

## Preuve exécutable

Après build d'Identity Web :

```bash
NVBES_IDENTITY_TEST_WEB_UI=1 NVBES_IDENTITY_TEST_SECURITY_KEY=1 pnpm nx run identity-service:test:https-browser-fixture
```

Le scénario `runtime-browser-hosted-security-key.mjs` utilise le site construit,
les services Identity/Account et le SDK actifs. Sa clé CTAP2 virtuelle utilise
le transport USB avec `hasResidentKey=false` et vérification utilisateur.
Il vérifie :

- enregistrement par l'interface et première assertion signée ;
- logout puis nouveau login par mot de passe, sans consentement avant la clé ;
- options serveur liées au credential enregistré avec UV obligatoire ;
- assertion sans `userHandle`, sans modification des options ou de la réponse ;
- refus du rejeu avant l'échange OAuth/DPoP et accès Account HTTP 200 ;
- credential non résident, compteur de signatures et absence d'erreur JavaScript ;
- écrans desktop/mobile sans débordement ni contrôle tronqué hors viewport.

Les captures de la fixture sont `/tmp/nvbes-security-key-desktop.png` et
`/tmp/nvbes-security-key-mobile.png`. Son nettoyage arrête ses processus et
supprime ses conteneurs. Ne pas cumuler ce scénario avec les modes Account,
réauthentification ou récupération de mot de passe : il exige une fixture neuve.

Le scénario SDK existant `runtime-browser-security-key.mjs` reste distinct et
conserve la vérification d'annulation avec révocation de la session primaire.

## Limites

Cette preuve couvre mot de passe puis clé, pas un login sans mot de passe après
saisie de l'email. Le login découvrable reste un autre parcours. La clé est
virtuelle : USB physique, NFC, application mobile et authentificateurs sans
vérification utilisateur ne sont pas validés ici. Aucun niveau AAL/FAPI ni
attestation matérielle n'est déduit de la résidence ou du transport.

Aucune migration, nouveau service, coût récurrent ou activation publique.
