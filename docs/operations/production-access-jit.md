# Accès humain à la production — PAM/JIT

## Principe

Il n’existe aucun accès SSH permanent à la production. Les règles Terraform ont
`enable_jit_ssh = false` et une liste vide par défaut. L’accès normal passe par
les API d’administration avec passkey, step-up, rôle minimal et élévation
temporaire. L’accès shell est réservé aux incidents que les APIs ne permettent
pas de traiter.

## Conditions d’ouverture

Une demande doit contenir :

- incident ou change ticket ;
- ressource exacte et commandes prévues ;
- rôle minimal demandé ;
- approbateur distinct du demandeur ;
- authentification AAL2 récente par passkey ou clé matérielle ;
- référence `audit://` immuable de l’événement d’authentification ;
- CIDR source fixe et vérifié ;
- durée maximale de 60 minutes ;
- confirmation qu’aucune donnée client ne sera copiée localement.

Security ou l’incident commander approuve les SEV1. Platform et le propriétaire
du domaine approuvent les changements planifiés. Le demandeur ne peut pas
approuver sa propre élévation.

## Ouverture JIT

1. Fournir au plan Terraform `enable_jit_ssh`, `ssh_allowed_ips`,
   `jit_access_ticket`, `jit_access_requester`, `jit_access_approver`,
   `jit_access_authentication_method`, `jit_access_authentication_event_ref`,
   `jit_access_authentication_verified_at` et `jit_access_expires_at`, sans
   modifier le code versionné.
2. Vérifier que le CIDR est un `/32`, que la clé SSH est matérielle ou adossée à
   un certificat court, et que l’utilisateur est nominatif.
3. Appliquer après approbation enregistrée.
4. Démarrer l’enregistrement de session et l’audit des commandes.
5. Programmer immédiatement le plan de fermeture ; ne pas attendre la fin de
   l’intervention.

Terraform refuse l’ouverture si le CIDR n’est pas un `/32`, si demandeur et
approbateur sont identiques, si le ticket manque, si l’authentification
WebAuthn date de plus de 15 minutes, si sa référence d’audit manque, ou si
l’expiration dépasse 60 minutes. Les comptes partagés, mots de passe SSH, accès
root direct, agent forwarding, port forwarding et clés sans expiration sont
interdits.

## Fermeture

1. Remettre `enable_jit_ssh = false` et `ssh_allowed_ips = []`.
2. Révoquer le certificat ou la clé temporaire.
3. Vérifier depuis l’extérieur que le port 22 est fermé.
4. Attacher les journaux de session, changements et preuves à l’incident.
5. Faire tourner tout secret consulté ou potentiellement exposé.

## Post-access review

Dans les 24 heures, un reviewer indépendant rapproche ticket, approbation,
fenêtre d’accès, commandes, audit applicatif et changements d’infrastructure.
Toute divergence déclenche un incident sécurité. Les activations break-glass et
élévations `enterprise.admin_elevation.granted` sont alertées par le SIEM.

## Preuve d’application avant activation production

Pour chaque release candidate, un opérateur et un reviewer indépendant exécutent
les cinq scénarios suivants sur l’environnement de production fermé au public :

1. Backoffice/support : un jeton AAL1 ou OTP est refusé, puis une passkey AAL2
   récente est acceptée.
2. Billing/gestion des secrets : un OTP est refusé pour une mutation Billing et
   pour une rotation de secret Developer ; une passkey AAL2 récente est acceptée.
3. Accès production : Terraform refuse une ouverture JIT sans événement
   `audit://` WebAuthn récent, puis accepte la demande complète.
4. Élévation temporaire : l’RPC refuse le contexte sans événement
   d’authentification et persiste l’événement accepté dans
   `enterprise.admin_elevation.granted`.
5. Break-glass : l’activation est refusée sans passkey/clé matérielle récente et
   l’acceptation produit `enterprise.break_glass.activated`.

Les refus, acceptations, identifiants d’événements d’authentification, hash de la
release testée et résultats du reviewer sont exportés dans le coffre de preuves.
La référence `vault://` et le statut de chacun des cinq scénarios sont ensuite
enregistrés dans `docs/compliance/external-security-assurance.json`. Le gate
`NVBES_PRODUCTION_RELEASE=1` interdit la release tant que cette preuve live
signée n’est pas complète et rattachée exactement à la release du pentest.
