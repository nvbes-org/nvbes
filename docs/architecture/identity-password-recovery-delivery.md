# Livraison durable des liens de récupération

La migration `0024_identity_recovery_deliveries.sql` ajoute une file PostgreSQL
spécifique aux emails contenant un lien secret. La file de notifications de
sécurité existante conserve son contrat sans secrets en clair.

## Transaction et chiffrement

La création du challenge, l'audit et l'insertion de la commande Email chiffrée
partagent une transaction. Un échec de validation ou de stockage annule tout.
La table de challenges conserve uniquement le hash du token. La nouvelle file
ne conserve ni le token, ni l'URL, ni le destinataire en clair.

AES-256-GCM utilise le jeu de clés déjà configuré pour Identity, avec un domaine
AAD distinct de TOTP, lié au principal et au challenge. Déplacer le ciphertext
vers un autre compte, challenge ou usage invalide son authentification. Les
versions active et précédente permettent de reprendre les envois pendant une
rotation. Cette séparation de domaine ne protège pas d'une compromission de
la clé partagée elle-même.

Les commandes sont bornées à 16 Kio avant chiffrement. Le lien expire après
quinze minutes selon l'horloge PostgreSQL. Le dispatcher revérifie le challenge,
le compte actif et le destinataire toujours vérifié avant son appel Email.
Un reset annule les envois en attente et efface leur ciphertext dans la même
transaction que les liens et sessions révoqués.

## Commandes opérateur

- `request-password-recovery` utilise `NVBES_IDENTITY_RECOVERY_EMAIL` et
  `NVBES_IDENTITY_RECOVERY_BASE_URL`. Il crée une demande durable et affiche
  uniquement `queued: true`. Il n'envoie pas d'email.
- `dispatch-password-recovery` traite un lot via le client gRPC Email existant
  et affiche uniquement des compteurs. Exécuter cette commande autorise les
  envois contenus dans ce lot ; elle n'est pas lancée automatiquement.

Les deux commandes nécessitent la configuration de base et de chiffrement
Identity ; le dispatcher nécessite aussi la configuration Email. Le mode
production impose les clés explicites existantes. Appliquer la migration avant
de démarrer le nouveau binaire. Configurer une destination web effective avant
tout usage réel : le [parcours HTTP/web](identity-password-recovery-http.md)
requiert son activation explicite et le routage du document Identity.

## Reprise et limites

- Au plus 16 prises en charge par lot, 8 tentatives par demande, timeout réseau
  de 20 secondes et lease de 60 secondes. Pas de boucle autonome.
- Après indisponibilité, reprise différée de 30 secondes, puis doublement borné
  à 1 920 secondes, toujours limitée par l'expiration du lien.
- Les leases expirées sont récupérables. La commande et sa clé d'idempotence
  restent identiques, y compris si Email a accepté avant la perte du commit
  de l'accusé. La déduplication relève du contrat Email : pas de promesse
  d'envoi réseau exactement une fois.
- L'état `accepted` signifie accepté par Email, pas livré dans la boîte mail.
- Une commande illisible, un reçu invalide, un refus d'authentification ou un
  conflit d'idempotence termine la demande en échec sans tentative de contourner
  la vérification. Restaurer une clé manquante ne ressuscite pas une demande
  dont le ciphertext a été effacé ; émettre un nouveau lien si nécessaire.
- Les traitements terminés n'ont plus de ciphertext, nonce ou version de clé.
  Le dispatcher purge au plus 32 métadonnées de plus de 30 jours et termine
  au plus 32 demandes obsolètes par lot. Sans exécution opérateur, ce nettoyage
  n'est pas une garantie de suppression à échéance.
- Un envoi déjà engagé peut parvenir après une révocation concurrente ; le lien
  consommé reste inutilisable. Les leases empêchent un ancien worker d'écraser
  le résultat d'une reprise.

Conserver l'ancienne clé pendant la validité des demandes qui l'utilisent ou
vider/annuler ces demandes avant son retrait. Aucun nouveau service, fournisseur,
scheduler ou coût fixe n'est ajouté. Les volumes réels Email et PostgreSQL
doivent encore être mesurés dans le plafond global de 30 EUR TTC/mois.

## Preuves et périmètre restant

Les tests utilisent les migrations réelles, des schémas privés, deux dispatchers
concurrents, un échec d'accusé en base, une rotation de clé et un serveur gRPC
Email local simulant une indisponibilité. Les liens expirés, consommés et les
destinataires devenus non vérifiés ne sont pas soumis au service Email.

Le [parcours HTTP/web](identity-password-recovery-http.md) décrit les protections
et quotas désormais raccordés. Restent les notifications après changement,
la cadence opérateur démontrée et la livraison
fournisseur. Cette file ne constitue pas un login par email ni une preuve MFA.
Les lots A–D restent ouverts.
