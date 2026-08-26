# Matrice de Regression V1

> **Statut : matrice produit historique à reconstruire pour le socle.** Les
> lignes Cloud/Drive ou Enterprise sont futures et ne doivent pas déclencher la
> restauration des applications archivées.

## Objectif

Lister les scenarios minimum a proteger avant lancement public.

## Parcours critiques

| Domaine        | Scenario                                                    | Niveau minimal        |
| -------------- | ----------------------------------------------------------- | --------------------- |
| Auth           | signup, verification email, login, logout                   | integration + E2E     |
| Sessions       | changement mot de passe invalide les sessions               | unit + integration    |
| Permissions    | role member/viewer/admin applique les bons droits           | unit + integration    |
| Upload         | creation upload session, upload, complete, activation objet | integration + E2E     |
| Upload         | upload expire ou checksum invalide                          | unit + integration    |
| Quotas         | quota mis a jour apres upload et bloque si depassement      | unit + integration    |
| Partage public | creation lien, acces public, expiration, revocation         | integration + E2E     |
| Audit          | evenements emis pour actions sensibles                      | integration           |
| Billing        | webhook signe, idempotent, reconciliation interne           | integration           |
| Billing        | affichage prochaine facture estimee                         | integration + UI      |
| Privacy        | export utilisateur/workspace                                | integration           |
| Privacy        | suppression compte/workspace et jobs associes               | integration           |
| Workers        | purge uploads expires, purge corbeille, snapshots usage     | unit + integration    |
| API publique   | cle API, scopes, rate limit et erreurs de base              | integration + E2E API |

## Etats UI a proteger

- drive vide;
- upload en cours;
- upload echoue;
- lien expire;
- lien revoque;
- quota presque atteint;
- quota depasse;
- permission refusee;
- fichier en quarantaine;
- billing echoue;
- trial expire;
- workspace suspendu;
- rate limit API atteint.
