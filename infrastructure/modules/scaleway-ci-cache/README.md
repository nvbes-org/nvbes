# Scaleway CI cache

Ce module ajoute un plan de cache isolé sans modifier les registries ni les
buckets produit existants :

- un projet Scaleway dédié ;
- un Container Registry privé pour les caches BuildKit `type=registry` ;
- un bucket Object Storage chiffré pour `sccache`, pnpm ou Nx ;
- une application IAM trusted pour `main`, `dev`, `staging` et `release/*` ;
- une application IAM séparée, limitée aux préfixes de branches ;
- deux API keys à rotation décalée par application ;
- un environnement GitHub protégé pour les branches trusted ;
- un environnement GitHub dédié aux pushes des autres branches.

Les deux slots tournent à mi-période l'un de l'autre. Terraform synchronise
vers GitHub le slot dont la prochaine rotation est la plus éloignée. Le slot
remplacé est donc inactif depuis une demi-période, ce qui évite de révoquer les
credentials d'un build déjà démarré.

Le bucket sépare les artefacts `sccache` en deux espaces :

- `trusted/rust/<plateforme>/<toolchain>/` partagé par les branches protégées ;
- `branches/<branche>/rust/<plateforme>/<toolchain>/` pour les autres pushes.

La bucket policy empêche l'identité de branche de lire ou d'écrire dans
`trusted/`. Les pull requests utilisent le backend GitHub Actions et ne
reçoivent aucun credential Scaleway, notamment lorsqu'elles proviennent d'un
fork ou de Dependabot.

Pendant le tout premier bootstrap, un push peut précéder la synchronisation des
secrets du nouvel environnement. Le job utilise alors le backend GitHub Actions
pour cette exécution seulement, sans dupliquer les tests, puis sélectionne
Scaleway automatiquement dès que Terraform a publié la paire active.

La rotation n'est effective qu'après un `terraform apply`. Le workflow
`rotate-ci-cache-credentials.yml` vérifie la stack chaque semaine dans
l'environnement protégé `production-bootstrap`. Si cet environnement impose
une review, un opérateur doit approuver l'exécution avant l'expiration annoncée.

Les secrets Scaleway transitent en clair dans le state Terraform, comme toute
ressource `scaleway_iam_api_key`. Le state bootstrap doit donc rester chiffré,
privé et inaccessible aux jobs applicatifs.
