# Politique relative à la protection des données à caractère personnel — nvbes Account

**Statut : projet d’information légale soumis à validation préalable à publication**

**Version : 2026-09-07**

**Date d’entrée en vigueur : `[À COMPLÉTER]`**

La présente politique est établie conformément au règlement (UE) 2016/679 du
27 avril 2016, ci-après le « **RGPD** », à la loi n° 78-17 du 6 janvier 1978
modifiée et, pour les opérations relevant de son champ, à l’article 82 de cette
loi.

Elle doit être complétée par les mentions signalées entre crochets et mise en
concordance avec les traitements, prestataires, transferts et mécanismes de
purge effectivement déployés avant sa publication.

## Article 1 — Objet et périmètre

La présente politique informe les personnes concernées des traitements de
données à caractère personnel réalisés dans le cadre de **nvbes Account**,
service de compte, d’identité et d’authentification destiné aux consommateurs.

Elle couvre :

- l’inscription, la vérification et l’administration du compte ;
- l’authentification, la récupération du compte et les facteurs de sécurité ;
- la gestion du profil, des adresses électroniques, des préférences, des
  sessions et des appareils ;
- l’autorisation d’applications au moyen de protocoles d’identité ou
  d’autorisation ;
- les communications relatives au compte, les demandes d’assistance et
  l’exercice des droits ;
- le portail web Account, ses stockages locaux et ses traceurs.

Elle ne couvre ni les contenus, fichiers, paiements ou traitements propres à un
autre produit nvbes, ni les traitements réalisés pour le compte d’un client
professionnel en qualité de sous-traitant. Ces traitements relèvent de
documents distincts.

## Article 2 — Définitions

Les termes « donnée à caractère personnel », « traitement », « personne
concernée », « responsable du traitement », « sous-traitant », « destinataire »,
« consentement » et « violation de données à caractère personnel » ont le sens
qui leur est attribué par l’article 4 du RGPD.

« **Account** » désigne exclusivement le service nvbes Account.

## Article 3 — Responsable du traitement

Le responsable des traitements décrits par la présente politique est :

- **Exploitant :** Rayane Guemmoud EI, entrepreneur individuel ;
- **Nom commercial :** OTAKIMI ;
- **Adresse professionnelle :** rue Auguste Renoir, 33400 Talence, France ;
- **Contact relatif à la protection des données :** privacy@nvbes.cloud ;
- **Délégué à la protection des données :** `[IDENTITÉ ET COORDONNÉES À
COMPLÉTER, OU MENTION DE L’ABSENCE DE DÉSIGNATION]`.

Dans cette politique, « nvbes » désigne Rayane Guemmoud EI lorsqu’il s’agit du
responsable du traitement. Nvbes et nvbes Account sont des noms de services.
Les coordonnées sont à confirmer selon les [mentions légales](./site-legal-notice.md).

Dans le cadre B2C d’Account, Rayane Guemmoud EI détermine les finalités et les moyens des
traitements nécessaires au fonctionnement, à la sécurité et à l’administration
du compte et agit en qualité de responsable du traitement.

Une application tierce autorisée par l’utilisateur peut agir en qualité de
responsable distinct pour les données qu’elle reçoit. Son identité, les
catégories de données demandées et les permissions correspondantes sont
présentées avant l’autorisation.

## Article 4 — Personnes concernées et provenance des données

Les personnes concernées sont :

- les visiteurs du portail Account ;
- les personnes qui entreprennent une inscription ;
- les titulaires d’un compte ;
- les personnes qui contactent l’assistance, le service juridique ou le
  service chargé de la protection des données.

Les données sont recueillies :

1. directement auprès de la personne concernée ;
2. à partir de son navigateur, de son terminal et de son utilisation
   d’Account ;
3. auprès d’une application ou d’un fournisseur d’identité à la demande de la
   personne concernée ;
4. auprès des prestataires techniques chargés de transmettre un statut de
   remise, de sécurité ou d’incident ;
5. à partir des bases de géolocalisation IP et de renseignement réseau
   effectivement configurées : `[SOURCES DE PRODUCTION À IDENTIFIER]`.

Lorsqu’une donnée est obtenue indirectement, l’information prévue à l’article
14 du RGPD est fournie au plus tard dans un délai d’un mois, lors du premier
contact ou avant la première communication à un destinataire, selon l’événement
qui intervient en premier, sauf exception légalement applicable.

## Article 5 — Catégories de données traitées

Selon les fonctions utilisées, nvbes traite les catégories suivantes.

### 5.1 Données d’identification et de profil

- identifiants internes du compte ;
- adresse électronique principale et adresses secondaires ;
- nom, prénom, nom d’affichage et nom d’utilisateur ;
- date de naissance, langue et préférences lorsqu’elles sont renseignées ;
- pays, région et région de données déterminés lors de l’inscription ;
- photographie de profil lorsqu’elle est ajoutée ;
- statut, dates de création, de vérification et de modification du compte.

### 5.2 Données d’authentification

- empreinte cryptographique non réversible du mot de passe ;
- clés publiques, identifiants et métadonnées de passkeys ou clés de sécurité ;
- secret TOTP chiffré, codes de récupération hachés et état des facteurs
  d’authentification ;
- jetons temporaires de vérification, de récupération et de changement
  d’adresse sous une forme protégée ;
- historique des modifications affectant les moyens d’authentification.

La donnée biométrique éventuellement utilisée localement par un terminal pour
déverrouiller une passkey n’est pas reçue par nvbes.

### 5.3 Données de session, d’appareil et de sécurité

- identifiants, dates, durée, activité et statut des sessions ;
- type d’appareil, navigateur, système d’exploitation et agent utilisateur ;
- capacité tactile, classes de processeur, de mémoire, d’écran et de profondeur
  de couleur, langue, fuseau horaire et signaux User-Agent Client Hints ;
- signaux d’intégrité du navigateur, notamment automatisation déclarée,
  disponibilité du stockage et cohérence des caractéristiques techniques ;
- adresse IP et données réseau nécessaires à la sécurité ;
- identifiant de terminal, signaux de confiance, événements, scores et
  décisions de risque ;
- journaux d’accès, d’audit, d’erreur et de sécurité.

### 5.4 Autorisations et applications

- identité de l’application ;
- permissions, portées et attributs demandés ;
- autorisations accordées, refusées ou révoquées ;
- identifiants ou empreintes de jetons, portées, dates d’émission, d’expiration
  et de révocation ; le secret d’un jeton n’est pas conservé en clair lorsqu’il
  doit être persisté ;
- clients d’autorisation ou intégrations créés par l’utilisateur, le cas
  échéant.

### 5.5 Communications, préférences et preuves

- préférences de notification et de prospection ;
- version et date d’acceptation des conditions contractuelles ;
- choix relatifs aux traceurs et traitements facultatifs, avec leur finalité,
  le texte présenté, sa version, la surface de collecte, la date et le retrait ;
- messages et pièces transmis à l’assistance ;
- pour les communications transactionnelles Account : adresse et nom du
  destinataire, expéditeur, objet, corps texte et HTML, lien ou code temporaire
  et données de l’opération concernée, en-têtes techniques et échéance d’envoi ;
- identifiants du message et du prestataire, statut, tentatives, dates, erreurs,
  rebonds, signalements de spam et inscriptions sur une liste de suppression.

### 5.6 Mesure d’usage et diagnostic

Lorsque le traitement correspondant est licitement activé :

- événements fonctionnels limités ;
- identifiants pseudonymisés ;
- pages ou fonctions utilisées ;
- informations techniques relatives à une erreur ou à la stabilité ;
- choix de consentement et identifiants de corrélation.

Les secrets d’authentification, jetons, mots de passe et adresses électroniques
en clair sont exclus des événements de mesure d’usage.

nvbes n’a pas pour finalité de collecter des catégories particulières de
données au sens de l’article 9 du RGPD au moyen d’Account.

## Article 6 — Finalités et bases juridiques

| Finalité déterminée                                                                                             | Données principales                                                          | Base juridique                                                                                                             |
| --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Créer, vérifier et administrer le compte                                                                        | Identification, profil, statut, région                                       | Article 6, paragraphe 1, point b), du RGPD                                                                                 |
| Authentifier la personne, maintenir ses sessions et permettre la récupération                                   | Authentification, session, appareil                                          | Article 6, paragraphe 1, point b), du RGPD                                                                                 |
| Gérer les facteurs de sécurité et les opérations sensibles                                                      | Facteurs, appareils, événements de sécurité                                  | Article 6, paragraphe 1, point b), du RGPD                                                                                 |
| Exécuter une autorisation demandée au bénéfice d’une application                                                | Identité de l’application, portées et attributs autorisés                    | Article 6, paragraphe 1, point b), du RGPD                                                                                 |
| Adresser les communications indispensables au compte                                                            | Adresse électronique, statut de remise, événement concerné                   | Article 6, paragraphe 1, point b), du RGPD                                                                                 |
| Fournir l’assistance relative au compte                                                                         | Identification, demande, éléments de diagnostic                              | Article 6, paragraphe 1, point b), du RGPD                                                                                 |
| Assurer la confidentialité, l’intégrité, la disponibilité et la résilience des traitements                      | Journaux, événements, appareils et réseau                                    | Article 6, paragraphe 1, point c), et article 32 du RGPD                                                                   |
| Détecter et prévenir les intrusions, fraudes, abus et prises de contrôle                                        | IP, appareil, événements et score de risque                                  | Article 6, paragraphe 1, point f), du RGPD — intérêt légitime à protéger les comptes et le Service                         |
| Établir, exercer ou défendre les droits de nvbes                                                                | Preuves contractuelles, journaux et correspondances pertinents               | Article 6, paragraphe 1, point f), du RGPD — intérêt légitime à assurer la preuve et la défense des droits                 |
| Répondre à une demande d’exercice de droits ou à une autorité compétente                                        | Identité, demande, justificatif proportionné, réponse                        | Article 6, paragraphe 1, point c), et articles 12 à 22 et 31 du RGPD                                                       |
| Adresser des communications de prospection facultatives                                                         | Adresse électronique, préférence et preuve du choix                          | Article 6, paragraphe 1, point a), du RGPD et article L. 34-5 du Code des postes et des communications électroniques       |
| Activer les traceurs et diagnostics facultatifs du navigateur                                                   | Identifiants, navigation et diagnostic                                       | Article 6, paragraphe 1, point a), du RGPD et article 82 de la loi du 6 janvier 1978                                       |
| Produire des statistiques techniques strictement minimisées nécessaires au pilotage et à la fiabilité d’Account | Événement fonctionnel, identifiant pseudonymisé et contexte technique limité | Article 6, paragraphe 1, point f), du RGPD — intérêt légitime à mesurer la fiabilité et l’usage des fonctions essentielles |

Les événements techniques émis côté serveur sont distincts des traceurs du
navigateur. Ils sont limités à l’achèvement d’une fonction, à un identifiant de
compte pseudonymisé, au pays ou à la région et à des identifiants techniques de
corrélation ; ils n’autorisent pas l’activation d’un traceur sans consentement.

Les traitements fondés sur l’intérêt légitime font l’objet d’une analyse
documentée de nécessité et de mise en balance. La personne concernée peut s’y
opposer dans les conditions de l’article 21 du RGPD et de l’article 13 de la
présente politique.

Une finalité facultative fondée sur le consentement demeure désactivée tant que
celui-ci n’a pas été valablement donné. Le retrait du consentement ne modifie
pas rétroactivement la base juridique du traitement.

## Article 7 — Caractère obligatoire ou facultatif

L’adresse électronique, le nom d’utilisateur, un moyen d’authentification, le
pays ou la région déterminé aux fins de disponibilité et de résidence des
données, ainsi que les données techniques strictement nécessaires à la sécurité
sont requis pour créer et utiliser le compte. L’impossibilité de vérifier un
pays pris en charge ou l’absence de ces données empêche la conclusion ou
l’exécution du service demandé.

Les autres informations de profil, la photographie, la prospection et les
mesures facultatives d’usage ou de diagnostic sont optionnelles. Leur refus
n’empêche pas l’utilisation des caractéristiques essentielles d’Account.

## Article 8 — Destinataires

Dans la limite de leurs attributions et du besoin d’en connaître, les données
peuvent être communiquées :

- au personnel habilité de nvbes soumis à une obligation de confidentialité ;
- aux prestataires d’hébergement, de base de données, de stockage, de réseau et
  de sauvegarde agissant en qualité de sous-traitants ;
- au prestataire d’envoi des communications transactionnelles ;
- aux prestataires de sécurité, d’observabilité, de diagnostic ou de support
  effectivement activés ;
- à une Application autorisée, dans la limite des attributs et permissions
  expressément accordés ;
- aux conseils professionnels, juridictions et autorités lorsque cette
  communication est légalement requise ou nécessaire à la défense d’un droit.

Les prestataires de production doivent être identifiés avant publication :

| Prestataire                                                   | Fonction                                                                            | Qualité       | Localisation et transferts                                                                                                                                                                               |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[HÉBERGEUR À CONFIRMER]`                                     | Hébergement, base de données, stockage et sauvegardes                               | Sous-traitant | `[À COMPLÉTER]`                                                                                                                                                                                          |
| `[PRESTATAIRE RÉSEAU À CONFIRMER]`                            | DNS, acheminement et protection réseau                                              | Sous-traitant | `[À COMPLÉTER]`                                                                                                                                                                                          |
| Scaleway SAS — Transactional Email (TEM) et Topics and Events | Acheminement des communications transactionnelles et remontée des statuts de remise | Sous-traitant | France (`fr-par`) ; traitement TEM annoncé au sein de l’Union européenne, sans transfert hors UE propre à TEM au 12 août 2026, sous réserve du DPA et de la liste des sous-traitants Scaleway en vigueur |
| PostHog, si activé                                            | Statistiques techniques ou mesure d’usage selon la configuration                    | Sous-traitant | `[RÉGION ET MÉCANISME À COMPLÉTER]`                                                                                                                                                                      |
| Sentry, si activé                                             | Diagnostic d’erreurs après consentement lorsqu’il est requis                        | Sous-traitant | `[RÉGION ET MÉCANISME À COMPLÉTER]`                                                                                                                                                                      |
| Grafana Labs, si activé                                       | Observabilité technique                                                             | Sous-traitant | `[RÉGION ET MÉCANISME À COMPLÉTER]`                                                                                                                                                                      |

La [liste publique des sous-traitants](./subprocessors.md) précise les données
confiées à chaque prestataire et les garanties de transfert applicables. Le
canal TEM est réservé aux communications transactionnelles Account et n’est pas
utilisé pour la prospection.

nvbes ne vend pas les données à caractère personnel.

## Article 9 — Transferts en dehors de l’Espace économique européen

La version publiée de la présente politique identifie les pays dans lesquels
les données sont traitées ou accessibles et le mécanisme applicable à chaque
transfert.

Lorsqu’un destinataire est situé dans un pays ne bénéficiant pas d’une décision
d’adéquation, le transfert est subordonné à une garantie conforme au chapitre V
du RGPD, notamment aux clauses contractuelles types adoptées par la décision
d’exécution (UE) 2021/914, complétées, lorsque cela est nécessaire, par une
évaluation du transfert et des mesures supplémentaires.

Une copie ou une description des garanties peut être demandée à
privacy@nvbes.cloud. Les informations protégées par le secret des affaires ou
une obligation de sécurité peuvent être occultées dans la mesure strictement
nécessaire.

Aucun transfert ne peut être réputé licite par la seule insertion de la
présente clause : les flux, pays, garanties et accès distants effectivement
utilisés doivent être documentés.

## Article 10 — Durées de conservation

Les durées maximales suivantes s’appliquent à compter de l’événement indiqué.
Elles doivent être traduites en règles de purge effectives avant publication.

| Données                                                                 | Conservation en base active                                                                               | Sort ou archivage                                                                                                                                                                             |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Inscription non vérifiée                                                | Sept jours à compter de l’inscription                                                                     | Suppression                                                                                                                                                                                   |
| Jeton de vérification et inscription en cours                           | Vingt-quatre heures à compter de l’émission                                                               | Expiration et suppression                                                                                                                                                                     |
| Jeton de réinitialisation du mot de passe                               | Trente minutes à compter de l’émission                                                                    | Expiration et suppression                                                                                                                                                                     |
| Compte vérifié et profil                                                | Durée de la relation contractuelle                                                                        | Suppression ou anonymisation dans les trente jours suivant sa fin, sous réserve des archives justifiées                                                                                       |
| Compte inactif                                                          | Vingt-quatre mois à compter de la dernière activité authentifiée, puis préavis de trente jours            | Clôture et application du régime de fin de contrat                                                                                                                                            |
| Photographie de profil                                                  | Jusqu’à son retrait ou à la fin du compte                                                                 | Suppression de l’objet actif dans les trente jours                                                                                                                                            |
| Session et cookie de session                                            | Trente jours au plus et expiration après douze heures d’inactivité                                        | Révocation ou suppression à l’expiration                                                                                                                                                      |
| Jeton d’accès OAuth                                                     | Quinze minutes à compter de l’émission                                                                    | Expiration ; maintien des seules traces de sécurité justifiées                                                                                                                                |
| Code d’autorisation OAuth                                               | Dix minutes à compter de l’émission                                                                       | Expiration et suppression                                                                                                                                                                     |
| Code d’appareil OAuth                                                   | Cinq minutes à compter de l’émission                                                                      | Expiration et suppression                                                                                                                                                                     |
| Jeton de rafraîchissement OAuth                                         | Trente jours au plus à compter de l’émission                                                              | Expiration, rotation ou révocation                                                                                                                                                            |
| Jeton de confiance du terminal                                          | Cent quatre-vingts jours au plus                                                                          | Expiration ou révocation anticipée                                                                                                                                                            |
| État de confiance du terminal                                           | Quatre-vingt-dix jours au plus                                                                            | Réévaluation ou suppression                                                                                                                                                                   |
| Export préparé pour téléchargement                                      | Vingt-quatre heures à compter de sa génération                                                            | Suppression du cache d’export                                                                                                                                                                 |
| Autorisation d’application                                              | Jusqu’à révocation, fin du compte ou invalidation de l’application                                        | Suppression de l’autorisation active ; conservation probatoire limitée selon la ligne relative aux journaux                                                                                   |
| Charge utile des communications transactionnelles Account               | Jusqu’à la réception d’un statut terminal, puis trente jours selon la configuration cible                 | Purge cryptographique du destinataire et du contenu ; une borne indépendante de la réception du statut terminal doit être implémentée avant publication                                       |
| Registre de cycle de vie et événements de remise                        | Quatre cents jours après le statut terminal ou le traitement de l’événement, selon la configuration cible | Suppression ; les données nécessaires à une liste de suppression active relèvent de la ligne suivante                                                                                         |
| Liste de suppression des communications                                 | Tant que l’adresse demeure invalide, inaccessible ou à l’origine d’un risque de remise répété             | Réexamen, déblocage puis suppression lorsque la mesure n’est plus nécessaire ; la durée maximale et la procédure de répercussion auprès de Scaleway doivent être finalisées avant publication |
| Journaux d’accès, d’authentification, de risque, d’audit et de sécurité | Douze mois à compter de l’événement                                                                       | Suppression, sauf extraction liée à un incident ou contentieux                                                                                                                                |
| Dossier d’incident ou de contentieux                                    | Durée de traitement du dossier                                                                            | Archivage restreint pendant cinq ans à compter de sa clôture, sauf prescription ou obligation différente                                                                                      |
| Preuve d’acceptation des CGU                                            | Durée du contrat                                                                                          | Archivage restreint pendant cinq ans à compter de sa fin                                                                                                                                      |
| Demande d’exercice de droits                                            | Durée nécessaire à son traitement                                                                         | Archivage restreint pendant cinq ans lorsqu’il est nécessaire d’établir le respect de l’obligation                                                                                            |
| Demande d’assistance                                                    | Durée de traitement                                                                                       | Trois ans à compter de la clôture, sauf nécessité probatoire plus longue dûment justifiée                                                                                                     |
| Consentement à la prospection                                           | Jusqu’au retrait                                                                                          | Preuve du retrait ou liste d’opposition pendant trois ans                                                                                                                                     |
| Choix relatif aux traceurs                                              | Cent quatre-vingt-trois jours à compter du choix                                                          | Renouvellement du choix ou suppression                                                                                                                                                        |
| Données de mesure facultative                                           | Treize mois au plus à compter de la collecte                                                              | Suppression ou anonymisation irréversible                                                                                                                                                     |

Les sauvegardes ne sont pas réutilisées à des fins opérationnelles. Les données
supprimées de l’environnement actif en disparaissent au plus tard à l’issue
d’un cycle de sauvegarde de trente jours, sauf gel légal ou incident dûment
documenté.

À l’expiration d’une durée, les données sont supprimées, anonymisées de manière
irréversible ou placées en archive intermédiaire à accès restreint lorsqu’une
obligation légale ou un besoin probatoire déterminé le justifie.

Scaleway prévoit que le contenu des messages TEM est automatiquement supprimé
après leur traitement. La durée exacte de ses métadonnées d’activité et de ses
listes de blocage doit être vérifiée dans le contrat et la configuration en
vigueur avant la publication de la présente politique.

## Article 11 — Cookies, stockages locaux et traceurs

Les opérations strictement nécessaires à la transmission d’une communication
ou à la fourniture d’Account expressément demandé par l’utilisateur sont mises
en œuvre sans consentement dans les limites de l’article 82 de la loi du
6 janvier 1978.

| Nom ou catégorie                                                   | Finalité                                              | Durée maximale                                                     | Régime                                                                                 |
| ------------------------------------------------------------------ | ----------------------------------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| `__Host-session` et variantes multi-compte                         | Authentification et maintien de la session            | Trente jours au plus, sous réserve de l’expiration pour inactivité | Strictement nécessaire                                                                 |
| `__Host-csrf_token` et variantes                                   | Prévention des requêtes intersites non autorisées     | Durée de la session correspondante                                 | Strictement nécessaire                                                                 |
| `__Host-registration_enrollment`                                   | Sécurisation de la vérification suivant l’inscription | Vingt-quatre heures au plus                                        | Strictement nécessaire                                                                 |
| `__Host-device`                                                    | Reconnaissance sécurisée d’un terminal                | Cent quatre-vingts jours au plus                                   | Strictement nécessaire à la sécurité, sous réserve de l’analyse documentée d’exemption |
| `nvbes.tracking-consent.v4` dans le stockage local                 | Conservation de la preuve locale du choix             | Cent quatre-vingt-trois jours                                      | Strictement nécessaire à la mémorisation du choix                                      |
| Traceurs PostHog, Sentry ou Grafana, si activés dans le navigateur | Mesure d’usage ou diagnostic                          | `[INVENTAIRE ET DURÉES À COMPLÉTER]`                               | Consentement préalable selon la finalité                                               |

Les traceurs non essentiels demeurent désactivés par défaut. L’acceptation et
le refus sont proposés dans des conditions équivalentes. Le choix est
granulaire par finalité et peut être retiré aussi facilement qu’il a été donné.

Lorsqu’un utilisateur est authentifié, son choix est associé au Compte et
synchronisé après authentification sur ses autres sessions. Le choix horodaté
le plus récent prévaut. L’acceptation, le refus et le retrait s’appliquent au
même périmètre de Compte. Cette synchronisation ne peut élargir la portée d’un
consentement ni réactiver un traitement retiré.

## Article 12 — Profilage et décisions automatisées

Account utilise des règles et scores de risque afin de détecter une activité
inhabituelle, de rejeter ou retarder une tentative, d’exiger une
authentification supplémentaire, de limiter une opération ou de révoquer une
session.

Ces mesures reposent notamment sur l’historique de session, le réseau, les
échecs d’authentification et les signaux de terminal détaillés à l’article 5.3.
Elles produisent un score et une décision immédiate d’autorisation, de
surveillance, de renforcement ou de blocage afin de protéger le Compte.

Une décision automatisée ne peut, à elle seule, entraîner la fermeture
définitive du Compte. Toute restriction persistante ou répétée peut faire
l’objet d’une contestation, de l’expression du point de vue de l’Utilisateur et
d’un examen humain auprès de support@nvbes.cloud. La qualification des décisions
au regard de l’article 22 du RGPD est documentée avant leur mise en production.

Toute modification de cette qualification impose une information préalable
précisant la logique générale, l’importance, les conséquences et les garanties
prévues par l’article 22 du RGPD.

## Article 13 — Droits des personnes concernées

Sous réserve des conditions et exceptions prévues par le RGPD, la personne
concernée dispose :

- du droit d’obtenir l’accès à ses données et une copie ;
- du droit d’obtenir la rectification des données inexactes ou incomplètes ;
- du droit d’obtenir l’effacement ;
- du droit d’obtenir la limitation du traitement ;
- du droit à la portabilité des données traitées par des moyens automatisés sur
  le fondement du consentement ou du contrat ;
- du droit de s’opposer, pour des raisons tenant à sa situation particulière,
  aux traitements fondés sur l’intérêt légitime ;
- du droit de s’opposer à tout moment et sans justification à la prospection
  directe, y compris au profilage qui lui est lié ;
- du droit de retirer son consentement à tout moment, sans affecter la licéité
  du traitement antérieur ;
- des droits afférents aux décisions individuelles automatisées dans les
  conditions de l’article 22 du RGPD ;
- du droit de définir des directives relatives à la conservation, à
  l’effacement et à la communication de ses données après son décès, dans les
  conditions de l’article 85 de la loi du 6 janvier 1978.

Les fonctions du compte permettent notamment la rectification du profil, la
gestion des sessions et autorisations, l’export et la demande de suppression.
Une demande peut également être adressée à privacy@nvbes.cloud ou à l’adresse
postale de l’article 3. Une modalité alternative de vérification proportionnée
est proposée lorsque la personne ne peut accomplir l’authentification renforcée
du parcours en ligne.

nvbes répond dans un délai d’un mois à compter de la réception de la demande.
Ce délai peut être prolongé de deux mois compte tenu de la complexité ou du
nombre de demandes ; la personne en est informée dans le premier mois.

Une demande est gratuite. En cas de doute raisonnable sur l’identité, nvbes
peut solliciter les seules informations supplémentaires nécessaires à sa
vérification. Tout justificatif d’identité éventuellement recueilli est
supprimé dès l’achèvement de la vérification, sauf conservation probatoire
distincte, nécessaire et dûment justifiée. Tout refus est motivé et indique les
voies de recours.

La personne concernée peut introduire une réclamation auprès de la Commission
nationale de l’informatique et des libertés, 3 place de Fontenoy, TSA 80715,
75334 Paris Cedex 07, ou auprès de l’autorité de contrôle compétente de son lieu
de résidence habituelle, de travail ou du lieu de l’infraction alléguée.

## Article 14 — Mineurs

Account est réservé aux personnes âgées d’au moins dix-huit ans et nvbes ne
cherche pas sciemment à collecter les données d’un mineur au moyen du Service.

Lorsqu’elle apprend qu’un Compte est détenu par un mineur, nvbes en suspend
l’utilisation, procède aux vérifications proportionnées et supprime les données
qui ne doivent pas être conservées en vertu d’une obligation légale. Le
représentant légal peut contacter privacy@nvbes.cloud.

## Article 15 — Sécurité

nvbes met en œuvre des mesures techniques et organisationnelles appropriées au
risque, comprenant notamment :

- le chiffrement des communications en transit ;
- le hachage non réversible des mots de passe ;
- le chiffrement des secrets d’authentification qui doivent pouvoir être
  restitués au service ;
- le hachage des codes de récupération ;
- le contrôle des accès selon le besoin d’en connaître ;
- la journalisation des opérations sensibles ;
- la révocation des sessions et facteurs compromis ;
- la surveillance, la sauvegarde et la gestion des incidents.

Ces mesures sont réévaluées en fonction de l’état des connaissances, des coûts
de mise en œuvre, de la nature du traitement et des risques pour les droits et
libertés.

## Article 16 — Violations de données

nvbes documente toute violation de données à caractère personnel. Lorsqu’une
violation est susceptible d’engendrer un risque pour les droits et libertés,
elle est notifiée à l’autorité de contrôle compétente conformément à l’article
33 du RGPD.

Lorsqu’elle est susceptible d’engendrer un risque élevé, les personnes
concernées sont informées dans les conditions de l’article 34 du RGPD, sauf
exception légalement applicable.

## Article 17 — Modification de la politique

La politique est révisée lorsque l’identité du responsable, une finalité, une
base juridique, une catégorie de données, un destinataire, un transfert, une
durée ou un droit fait l’objet d’une modification substantielle.

Une évolution purement technique qui ne modifie aucun de ces éléments ne
requiert pas une nouvelle version de la politique.

Toute modification substantielle est portée activement à la connaissance des
personnes concernées avant sa prise d’effet lorsque le droit applicable
l’exige. Une nouvelle finalité fondée sur le consentement demeure inactive
jusqu’à l’obtention d’un consentement valable.

Les versions successives sont datées et archivées.

## Article 18 — Contact

- **Exercice des droits et protection des données :** privacy@nvbes.cloud
- **Assistance relative au compte :** support@nvbes.cloud
- **Questions juridiques :** legal@nvbes.cloud
