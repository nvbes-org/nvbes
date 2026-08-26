# Note de marché — Base de données serverless européenne

> **Statut : recherche de marché, sans autorité d'achat ou de roadmap.** Toute
> décision fournisseur reste soumise au contrat FinOps global 20/30 EUR TTC et à
> la [direction produit V1](nvbes-product-strategy.md).

Date de l'analyse : 2026-08-02.

Statut : exploration produit. Ce document consigne une hypothèse de marché ; il ne constitue ni une ADR, ni une décision de construire ou de commercialiser le produit.

## Hypothèse

Il existe une demande crédible pour une offre combinant :

- PostgreSQL réellement compatible ;
- calcul serverless avec mise à zéro à l'inactivité ;
- société, contrôle juridique, données, sauvegardes et opérations situés dans l'Union européenne ;
- expérience développeur comparable à Neon ou Supabase ;
- coûts faibles et prévisibles pour les petites charges.

La demande ne porte pas seulement sur une base « hébergée en Europe ». Les acheteurs sensibles à la souveraineté recherchent aussi un fournisseur européen, une chaîne de sous-traitance documentée et une exposition minimale aux juridictions extraterritoriales.

## Signaux de marché

- En 2025, 53 % des entreprises de l'Union européenne achetaient des services cloud et 45,5 % des entreprises utilisatrices du cloud y hébergeaient leur base de données.
- La Commission européenne a lancé en 2025 un marché de 180 millions d'euros sur six ans pour l'achat de services cloud souverains.
- Neon, Supabase et les offres serverless des hyperscalers valident la demande mondiale pour une base élastique sans exploitation manuelle.
- Le lancement de Serverless SQL par Scaleway valide directement l'intérêt d'une déclinaison européenne de ce modèle.
- L'offre européenne reste fragmentée : Scalingo et Clever Cloud proposent du PostgreSQL européen mais peu d'élasticité à zéro ; Scaleway fournit l'infrastructure serverless, mais avec une expérience encore orientée infrastructure et certaines limitations PostgreSQL.

Sources :

- [Eurostat — utilisation du cloud par les entreprises en 2025](https://ec.europa.eu/eurostat/de/web/products-eurostat-news/w/ddn-20260203-1)
- [Commission européenne — marché Cloud III souverain](https://commission.europa.eu/news-and-media/news/commission-moves-forward-cloud-sovereignty-eur-180-million-tender-2025-10-10_en)
- [Scaleway Serverless SQL](https://www.scaleway.com/en/serverless-sql-database/)
- [Tarifs Scalingo](https://scalingo.com/fr/pricing)
- [PostgreSQL managé Clever Cloud](https://www.clever.cloud/developers/doc/addons/postgresql/)

## Segments susceptibles de payer

1. SaaS européens servant des secteurs réglementés : santé, finance, juridique et secteur public.
2. Éditeurs voulant réduire l'exposition de leur chaîne de sous-traitance au CLOUD Act.
3. Startups et petites équipes recherchant une base à coût presque nul avant l'acquisition de trafic.
4. Agences et plateformes créant des environnements temporaires de développement ou de preview.
5. Plateformes ayant besoin d'une base isolée par client, projet ou workspace.

## Attentes minimales du produit

- Contrôle juridique européen et résidence UE vérifiable pour les données, sauvegardes, logs et opérations.
- PostgreSQL compatible avec les pilotes, ORM, migrations et outils standards.
- Scale-to-zero et reprise suffisamment rapides pour les usages interactifs.
- Pool de connexions adapté aux Functions et Containers.
- Plafond de dépense, quotas et alertes budgétaires.
- Sauvegardes, restauration testable et PITR pour les offres de production.
- API, CLI, Terraform, métriques et journaux d'audit.
- Réseau privé ou restrictions réseau robustes.
- Import et export simples depuis PostgreSQL, Neon, Supabase et RDS.
- Branches ou copies éphémères pour les previews, si l'économie du produit le permet.

## Opportunité possible pour nvbes

L'opportunité n'est pas de développer un nouveau moteur de base de données. Le coût, le risque opérationnel et les exigences de durabilité seraient disproportionnés.

Une piste plus réaliste serait une couche produit provisoirement nommée **nvbes Database**, construite sur Scaleway Serverless SQL :

- création et cycle de vie des projets et bases ;
- organisations, workspaces, rôles et permissions ;
- secrets et chaînes de connexion ;
- quotas, plafonds et facturation ;
- migrations, exports et restauration ;
- métriques, audit et preuves de résidence ;
- intégration native avec nvbes Functions, Containers et Jobs.

Scaleway resterait responsable du moteur et de l'infrastructure. nvbes apporterait l'expérience développeur, la gouvernance, la conformité et l'intégration entre produits.

Cette piste dépend toutefois de la faisabilité contractuelle et technique de l'automatisation ou de la revente de Serverless SQL. Elle ne doit pas être présentée comme une offre prévue avant vérification des API, quotas, conditions commerciales, responsabilités de support et marges.

## Risques

- Scaleway propose déjà directement le service sous-jacent.
- La souveraineté seule peut susciter de l'intérêt sans produire une volonté de payer.
- Une base externe à Scaleway peut réduire le coût facial tout en augmentant latence, complexité réseau et charge opérationnelle.
- La durabilité, les sauvegardes et le support transforment rapidement un produit peu coûteux en activité d'infrastructure exigeante.
- Une compatibilité PostgreSQL partielle peut bloquer des usages existants, notamment les verrous consultatifs et certaines extensions.

## Validation recommandée

Avant toute conception détaillée :

1. Interroger au moins 15 SaaS B2B européens, dont une majorité dans des secteurs réglementés.
2. Tester séparément l'attrait de la souveraineté, du scale-to-zero, du prix prévisible et des environnements de preview.
3. Obtenir au moins 5 clients pilotes prêts à payer entre 15 et 50 euros par mois.
4. Vérifier si ces clients préfèrent acheter une couche nvbes ou utiliser directement Scaleway.
5. Valider les conditions techniques et commerciales avec Scaleway avant toute promesse publique.

L'hypothèse sera considérée comme infirmée si les prospects valorisent la souveraineté mais refusent de payer au-delà du prix de l'infrastructure brute, ou s'ils préfèrent administrer directement leur compte Scaleway.
