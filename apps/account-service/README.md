# Account service

Runtime V1 propriétaire des profils, préférences, équipes, memberships et du
cycle de vie des données Account. Il accepte exclusivement les access tokens
RS256 émis par Identity pour l'audience Account.

Les requêtes protégées vérifient le profil `at+jwt`, l'issuer exact (sans retirer
son slash final), l'audience `nvbes-account-service`, les scopes et les dates,
puis consultent l'introspection Identity via le SDK Rust partagé. Un token
inactif donne 401 ; une panne ou une réponse incohérente donne 503. Aucun cache
positif ni repli sur la seule signature n'est utilisé. Les tokens liés à DPoP
restent refusés en Bearer jusqu'à l'intégration de la vérification des preuves.

Outre la clé publique, l'issuer et le key ID, configurer
`NVBES_ACCOUNT_IDENTITY_RESOURCE_CLIENT_ID` et
`NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET` avec une entrée Account du registre
`NVBES_IDENTITY_RESOURCE_SERVERS_JSON` d'Identity. Le helper de développement
prépare ces valeurs localement, avec un secret distinct de Billing. Un registre
personnalisé exige des credentials explicitement cohérents.

Les actions d'export et de fermeture qui demandent un step-up vérifient son
échéance au moment de l'action. La seule présence de `totp` ou `webauthn` dans
un ancien token ne suffit pas. Une connexion primaire passkey récente reste
recevable pendant cinq minutes, dans la limite de l'expiration du token.

Les exports sont construits par le job `process-privacy-jobs` et expirent après
24 heures. Une fermeture dispose d'un délai d'annulation de sept jours avant
redaction des données Account. Les autres domaines restent responsables de
leurs propres fragments et suppressions.

Le runtime ne fournit aucune inscription, credential, abonnement ou capacité
Enterprise.

Account décide également de l'accès aux comptes facturés : propriétaire d'un
profil personnel actif ou propriétaire d'une équipe active. Le endpoint interne
`POST /internal/v1/billing/authorize` est activé uniquement par
`NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET` (32 octets aléatoires en hexadécimal
minuscule, partagé exclusivement avec Billing). Le helper local prépare ce
secret séparément des credentials Identity. Voir le
[contrat d'autorisation Billing](../../docs/architecture/billing-account-authorization.md).
