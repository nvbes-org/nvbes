# API Publique V1 - Compatibilite

> **Statut : politique future pour une API produit non sélectionnée.** Elle ne
> crée aucune obligation de compatibilité publique pendant le socle V1.

## Garantie V1

`/v1` est stable apres lancement public. Un client conforme aux guides V1 doit continuer a fonctionner pendant toute la duree de support V1.

## Changements Compatibles

Sont compatibles:

- ajout d'un endpoint;
- ajout d'un champ optionnel dans une reponse;
- ajout d'une valeur enum documentee comme extensible;
- clarification de `message` dans une erreur;
- ajout de headers non obligatoires;
- augmentation de limites de plan.

## Changements Incompatibles

Sont incompatibles dans `/v1`:

- suppression ou renommage d'un endpoint;
- suppression ou renommage d'un champ de reponse;
- rendre obligatoire un champ de requete auparavant optionnel;
- changer le type JSON d'un champ existant;
- changer la semantique d'un scope;
- reduire une limite contractuelle sans clause de fair use ou incident;
- changer un code d'erreur stable;
- exposer une nouvelle exigence d'auth qui casse les clients existants.

## Deprecation

Une deprecation publique doit inclure:

- entree dans [public-api-v1-changelog.md](public-api-v1-changelog.md);
- champ `deprecated: true` dans l'OpenAPI si applicable;
- header `Deprecation`;
- header `Sunset` quand une date de retrait existe;
- alternative recommandee;
- delai minimum de 90 jours avant retrait public.

## Nouvelle Version

Creer `/v2` si un changement incompatible est necessaire. `/v1` reste disponible pendant la fenetre de migration annoncee.

## OpenAPI

Chaque release publique archive une spec immuable:

```text
docs/api/openapi/drive-public-v1.openapi.json
```

La spec doit etre generee depuis le code et verifier:

- presence des endpoints `/v1`;
- schemas de reponse d'erreur;
- security schemes;
- tags `public-api`;
- absence de route de creation d'API key legacy publique.
