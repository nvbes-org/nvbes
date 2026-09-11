# Bascule du dépôt en public

## Périmètre

Ce runbook concerne uniquement la visibilité du dépôt source
`nvbes-org/nvbes`. Il ne constitue ni un lancement commercial, ni une mise en
production, ni l'entrée en vigueur des documents contractuels présents sous
`docs/legal`.

## Divulgations assumées

La bascule rend immédiatement consultables et clonables :

- l'intégralité de l'historique Git et les branches rendues publiques par GitHub ;
- l'architecture, l'infrastructure déclarative, les plans et les prototypes archivés ;
- l'identité professionnelle, le SIREN et l'adresse professionnelle abrégée
  présents dans les projets de documents légaux ;
- les auteurs, adresses de commit et dates contenus dans l'historique Git.

Si l'une de ces informations ne doit pas être publique, arrêter la procédure.
Une réécriture d'historique doit être décidée et coordonnée avant la bascule.

## Préconditions dans le dépôt

- [x] texte intégral `AGPL-3.0-only` et exception MIT documentée ;
- [x] politique de sécurité et canal privé de signalement ;
- [x] guide de contribution, gouvernance et modèles GitHub ;
- [x] CI de PR externe sur runners GitHub-hosted, sans secret ;
- [x] workflow sécurité actif et actions tierces épinglées par SHA ;
- [x] historique Gitleaks revu et exceptions limitées à des empreintes précises ;
- [x] environnements de production limités à `main`, sans bypass administrateur ;
- [x] Dependabot alerts et mises à jour de sécurité activés ;
- [x] PR de préparation fusionnée sur `main` après validation locale complète ;
- [ ] jobs GitHub-hosted exécutés après levée de la restriction de facturation du
      dépôt privé (aucune étape de la PR de préparation n'a pu démarrer) ;
- [ ] sauvegarde miroir du dépôt et export des paramètres GitHub effectués ;
- [ ] confirmation explicite des divulgations ci-dessus par le propriétaire.

## Séquence de bascule

1. Suspendre les merges et vérifier que `main` correspond au SHA de la PR
   validée.
2. Exporter les paramètres GitHub, la liste des environnements et leurs règles.
3. Passer `nvbes-org/nvbes` en public dans les paramètres GitHub.
4. Activer immédiatement Dependency Graph, Dependabot alerts, Dependabot
   security updates, Code scanning, Secret scanning et Push protection.
5. Appliquer les rulesets versionnés sous `.github/rulesets` : aucune suppression
   ou force-push, PR obligatoire, routage CODEOWNERS, commits signés, historique
   linéaire, `ci-gate` et contrôles sécurité requis.
6. Exiger l'approbation des workflows pour tous les contributeurs externes.
7. Vérifier que chaque environnement `production-*` refuse toute branche autre
   que `main`, bloque le bypass administrateur et conserve ses secrets privés.
8. Ouvrir une PR depuis un fork de test et prouver que les jobs s'exécutent sur
   GitHub-hosted sans accès aux secrets ni aux runners self-hosted.
9. Relancer Gitleaks sur `--all`, les contrôles de licences, OSV, Trivy et CodeQL.

## Retour arrière

Repasser le dépôt en privé réduit l'accès futur mais ne retire aucune copie déjà
clonée. La confidentialité ne peut donc pas être restaurée. Une fuite de secret
exige révocation, rotation et audit, indépendamment de la visibilité du dépôt.
