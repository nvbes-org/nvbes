# DPoP dans Account et Billing

Les API acceptent les jetons liés à une clé sous `Authorization: DPoP`, avec une
preuve ES256 dans un unique header `DPoP`. Un tel jeton reste interdit en Bearer.
Une preuve ne transforme pas un jeton Bearer en jeton lié à une clé.

Le vérificateur partagé `nvbes-dpop`, feature `resource-server`, contrôle la
signature, la clé publique P-256, `typ`, `htm`, `htu`, `iat`, `ath` et l'empreinte
`cnf.jkt` du jeton. La preuve est bornée à 16 Kio et son `jti` à 256 caractères.
Les clés privées, courbes incohérentes, coordonnées non canoniques et paramètres
JOSE critiques non pris en charge sont refusés. Ces contrôles suivent le profil
ES256 de la [RFC 9449](https://www.rfc-editor.org/rfc/rfc9449.html#section-4.3).

## Origine de confiance

Configurer `NVBES_ACCOUNT_PUBLIC_ORIGIN` et `NVBES_BILLING_PUBLIC_ORIGIN` avec
l'origine publique canonique de chaque API : HTTPS, ou HTTP loopback local.
Le chemin est celui de `OriginalUri` dans Axum ; query et fragment ne participent
pas à la comparaison DPoP. Les headers Host et proxy ne déterminent pas l'URL
attendue. Un proxy doit préserver le chemin public jusqu'à l'API.

Sans origine configurée, les requêtes DPoP ne sont pas autorisées et donnent 503.
Les requêtes Bearer non liées à une clé conservent leur parcours existant.
Les lanceurs locaux préparent les origines loopback à partir des ports Account
et Billing ; un environnement public exige une configuration explicite.

## Révocation et rejeu

Le JWT est validé localement et sa preuve est vérifiée avant l'introspection
Identity. Une réponse active doit retrouver exactement ses claims, dont `cnf`
et le type DPoP. Ensuite seulement, l'API consomme la preuve dans sa propre base
PostgreSQL, avant d'entrer dans le handler métier. Billing conserve en plus
l'autorisation Account du compte facturé.

La migration 0002 crée `resource_dpop_replays` dans chacune des deux bases.
Elle ne partage aucune table avec Identity. Le registre contient les empreintes
de clé et de `jti`, le compartiment de stockage et l'expiration, jamais la preuve
ni le jeton. Un verrou transactionnel par compartiment et une contrainte unique
empêchent deux répliques d'accepter le même couple clé/`jti`.

Le runtime conserve au plus 64 compartiments de 256 marqueurs : 16 384 lignes
logiques par API. Une consommation nettoie les lignes expirées de son compartiment.
La fenêtre est `iat ± 300 secondes`, avec conservation jusqu'à `iat + 301` pour
couvrir les preuves datées dans le futur. L'admission revérifie l'expiration du
jeton et de la preuve ; l'INSERT vérifie aussi l'heure PostgreSQL afin qu'un
nettoyage anticipé par l'horloge de la base n'autorise pas un rejeu.

Les consommations sont bornées à 16 simultanées et deux secondes. Une saturation,
une migration absente ou une panne du registre donne 503 `dpop_unavailable`,
sans repli en mémoire ou sur la seule signature. Une preuve invalide ou rejouée
donne 401. Les erreurs d'authentification annoncent les schémas acceptés via
`WWW-Authenticate`.

Une preuve consommée ne redevient pas disponible si le handler métier échoue.
Chaque nouvelle tentative doit signer une nouvelle preuve ; la clé d'idempotence
métier reste indépendante. Le registre survit aux redémarrages. Aucun Redis ni
job de nettoyage supplémentaire n'est requis. Le coût réel des écritures,
du WAL, du vacuum et de la charge reste à mesurer avant exposition publique.

## Preuves et limites

`pnpm nx run identity-service:test:resource-runtimes` lance les tests de la
primitive partagée puis les trois vrais binaires avec migrations isolées.
Des preuves Node ES256 accompagnent des tokens réellement émis par Identity.
Le parcours vérifie mauvaise clé/méthode/URL/hash/date, downgrade Bearer,
rejeu concurrent, redémarrage, panne du registre, saturation complète,
récupération des places expirées et révocation par logout.

Les suites PostgreSQL Identity, Account et Billing restent des gates de régression.
Cette preuve utilise HTTP loopback ; elle ne remplace pas les essais navigateur
HTTPS, clients natifs ou charge. Le SDK navigateur PAR/DPoP reste à intégrer.
Les nonces émis par les serveurs de ressources ne sont pas activés dans ce profil.
La prise en charge du header DPoP sur PAR reste aussi à compléter côté Identity ;
la liaison `dpop_jkt` et la preuve au token endpoint ne suffisent pas à clore
l'intégralité du lot D. Aucune certification FAPI/AAL n'est revendiquée.
