# Revue des secrets avant ouverture publique

## Conclusion

Le 11 septembre 2026, Gitleaks 8.30.1 a analysé les 781 commits accessibles
avec `--all` et l'arbre courant. Les 28 détections historiques et 17 détections
de l'arbre ont été revues. Elles correspondaient à des exemples de
documentation, des UUID de démonstration, des valeurs de test déterministes ou
des placeholders de configuration. Aucun secret de production exploitable n'a
été identifié.

Les exemples actuels ont été reformulés ou vidés pour que l'arbre courant passe
sans exception. Les 28 détections historiques revues sont identifiées de façon
granulaire dans [`.gitleaksignore`](../../.gitleaksignore) par leur empreinte
commit, chemin, règle et ligne. Une nouvelle détection n'est donc pas masquée
par une exclusion large de chemin ou de règle.

## Classification des détections historiques

| Catégorie                                           | Chemins concernés                                        | Classification                  |
| --------------------------------------------------- | -------------------------------------------------------- | ------------------------------- |
| Jetons et clés d'API dans la documentation d'agents | `.agents/skills/**`                                      | placeholders pédagogiques       |
| Chiffrement HTTP                                    | `libs/ts/http-client/src/**.test.ts`                     | clés de test déterministes      |
| API Platform Operations                             | `docs/operations/platform-operations-api.md`             | UUID de démonstration           |
| Configurations Terraform                            | `infrastructure/environments/*/terraform.tfvars.example` | placeholders non fonctionnels   |
| Developer archivé                                   | `apps/developer-service/src/**contract_tests.rs`         | secrets et préfixes de fixtures |
| MFA                                                 | `docs/architecture/identity-mfa-recovery.md`             | exemple conceptuel OAuth        |

## Reproduction

```bash
gitleaks git . --log-opts='--all' --redact
gitleaks dir . --redact
```

La publication reste conditionnée à la révocation immédiate de toute
credential qui serait découverte ultérieurement. Supprimer seulement une
valeur de Git ne révoque jamais le secret correspondant.
