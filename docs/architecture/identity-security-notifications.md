# Notifications de sécurité Identity

## Périmètre livré

Les événements MFA génération/début/remplacement/abandon alimentent
`identity_security_notifications` dans la même transaction que l'audit et l'outbox.
La migration 0023 crée la file. Tous les identifiants email vérifiés au moment
de l'événement sont figés, jusqu'à seize destinataires ; au-delà, l'opération
est refusée et sa transaction annulée, sans omission silencieuse.

Chaque destinataire reçoit une commande AccountSecurity distincte avec une clé
d'idempotence événement + SHA-256 de l'adresse normalisée. Le contenu et
l'échéance (occurrence + 24 heures) ne changent pas pendant les reprises.
Une adresse ajoutée ou modifiée après l'événement ne reçoit pas rétroactivement
l'alerte. Les commandes ne contiennent ni code MFA, ni jeton, ni mot de passe.

Les adresses vérifiées manquantes produisent `no_recipient`. Les événements
antérieurs à la migration produisent `no_snapshot`, car leur destinataire
historique ne peut pas être déduit de l'adresse actuelle. Ces états exigent une
revue opérateur, sans tentative automatique vers une adresse non vérifiée.

## Exécution bornée

Le binaire expose `dispatch-security-notifications`. Il utilise DATABASE_URL,
NVBES_EMAIL_GRPC_ENDPOINT et NVBES_EMAIL_GRPC_AUTH_TOKEN avec la configuration
d'environnement existante. Le service Email doit accepter les valeurs Protobuf
5 à 8 avant cette exécution. La commande n'est pas lancée automatiquement par
le serveur HTTP et aucun scheduler ni infrastructure n'est ajouté.

Une invocation réclame au plus seize notifications. Chaque tentative est bornée
à vingt secondes et protégée par un bail de soixante secondes persisté avant
l'appel réseau. SKIP LOCKED répartit le travail concurrent ; un token de bail
empêche une réponse ancienne d'écraser une nouvelle tentative. Un processus
interrompu laisse une tentative récupérable après expiration du bail.

Les indisponibilités sont reprises jusqu'à huit tentatives, avec délais de
30, 60, 120, 240, 480, 960 puis 1920 secondes. L'opérateur doit relancer une
invocation lorsque le travail est dû. Les conflits d'idempotence, refus du caller
et commandes/reçus invalides sont terminaux. L'épuisement et l'expiration ne
sont jamais comptés comme des acceptations réussies. Un résultat réseau inconnu
peut avoir été accepté par Email : la reprise conserve donc la même commande.

## Statuts, données et exploitation

`accepted` signifie uniquement qu'Email a accepté la commande. L'identifiant du
message et sa date d'acceptation sont conservés ; la livraison et les bounces
restent sous la responsabilité d'Email. `published_at` de l'outbox métier n'est
pas modifié : d'autres consommateurs peuvent encore avoir à traiter l'événement.

La commande JSON contenant l'adresse est effacée dès l'état terminal. Les
métadonnées terminales sont nettoyées après trente jours, par lots de trente-deux
au début de chaque invocation. La suppression d'un principal supprime aussi ses
notifications. Les compteurs de sortie et les codes d'échec n'exposent pas les
adresses ou contenus. Ne pas journaliser les commandes pendant leur attente.

L'opérateur suit les lignes pending/sending dues, failed, expired, no_recipient
et no_snapshot, puis utilise receipt_id pour rapprocher le statut Email.
Une absence de notification ne transforme pas une récupération en authentification
forte ; les règles de reconnexion et de révocation restent inchangées.

## Validation et limites

Les tests utilisent PostgreSQL réel avec migrations. Le transport Email est
substitué dans les tests du dispatcher : concurrence, idempotence par destinataire,
reprise après disparition d'un processus, changement d'adresse, bail périmé,
reçus incohérents, absence d'adresse, expiration et épuisement sont couverts.
Une vraie génération/consommation MFA vérifie l'enqueue et le rollback de toute
la mutation quand la file échoue. La rétention conserve l'audit métier.

La cible Nx `identity-service:test:email-runtime` utilise un worker Email réel,
gRPC, deux schémas PostgreSQL isolés et le fournisseur local test-capture.
Une génération MFA réelle traverse la file ; une acceptation dont Identity perd
le reçu est rejouée après redémarrage forcé d'Email. Le même reçu est retrouvé,
avec un seul message, une seule tentative fournisseur et une capture rapprochée
du registre Email. Le refus d'un mauvais jeton producteur et l'absence des secrets
MFA dans le contenu sont vérifiés. Le processus enfant n'hérite d'aucune variable
du fournisseur réel ; son arrêt et la suppression des captures sont automatiques.

La capture prouve `provider_accepted`, pas `delivered` dans une boîte réelle.
La livraison fournisseur externe reste à valider dans les gates de mise en
service autorisés. Aucun email réel ou déploiement n'est effectué ici. La
cadence d'exploitation et la preuve FinOps globale restent à finaliser avant
ouverture publique ; les limites par invocation ne prouvent pas à elles seules
le plafond de 30 EUR TTC par mois.
