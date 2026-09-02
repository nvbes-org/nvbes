# Scaleway CI cache

Ce module ajoute un plan de cache isolé sans modifier les registries ni les
buckets produit existants :

- un projet Scaleway dédié ;
- un Container Registry privé pour les caches BuildKit `type=registry` ;
- un bucket Object Storage chiffré pour `sccache`, pnpm ou Nx ;
- une application IAM limitée au projet cache ;
- deux API keys à rotation décalée ;
- un environnement GitHub protégé contenant la paire active et les endpoints.

Les deux slots tournent à mi-période l'un de l'autre. Terraform synchronise
vers GitHub le slot dont la prochaine rotation est la plus éloignée. Le slot
remplacé est donc inactif depuis une demi-période, ce qui évite de révoquer les
credentials d'un build déjà démarré.

La rotation n'est effective qu'après un `terraform apply`. Le workflow
`rotate-ci-cache-credentials.yml` vérifie la stack chaque semaine dans
l'environnement protégé `production-bootstrap`. Si cet environnement impose
une review, un opérateur doit approuver l'exécution avant l'expiration annoncée.

Les secrets Scaleway transitent en clair dans le state Terraform, comme toute
ressource `scaleway_iam_api_key`. Le state bootstrap doit donc rester chiffré,
privé et inaccessible aux jobs applicatifs.
