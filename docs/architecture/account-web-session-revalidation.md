# Revalidation d'une page Account ouverte

Account relit son profil auprès du resource server toutes les 60 secondes de
présence visible et en ligne. Cette lecture traverse les contrôles DPoP et
d'introspection existants : le navigateur ne déduit pas lui-même qu'une session
Identity a été révoquée.

La périodicité commence après une lecture réussie et repart après chaque réponse.
Une requête lente n'en déclenche pas une seconde. Focus, retour en visibilité et
retour en ligne demandent une lecture immédiate, regroupée par le contrôleur
avec toute lecture déjà en cours. Les pages cachées/hors ligne ne déclenchent
pas de vérification périodique ; l'expiration locale du jeton reste applicable.

Pendant une lecture, le profil précédent est masqué. Un refus ou une panne
efface les accès en mémoire et arrête les tentatives, selon la politique du
contrôleur existant. Fermeture, logout et départ du document annulent les timers ;
le départ retire aussi les écouteurs. Une réponse tardive ne réouvre pas la page.

## Portée et coût

Le délai nominal de détection est d'une minute plus la durée de la requête.
Le timeout réseau existant est de dix secondes. Un navigateur suspendu ou un
client hors ligne ne permet pas de promettre cette échéance ; au retour en ligne
et en visibilité, la lecture est immédiate. L'introspection des appels API
continue d'appliquer la révocation indépendamment de cet effacement d'interface.

La vérification périodique ajoute au plus une lecture par minute et par document
visible, jusqu'à expiration de son accès. Les événements de retour peuvent ajouter
des lectures. Aucune connexion permanente, renouvellement de token, base ni
service supplémentaire. Cela ne démontre pas à lui seul le budget mensuel global :
le nombre de pages ouvertes et le coût des appels restent à mesurer pour FinOps.

Ce mécanisme n'est pas OpenID Connect Back-Channel Logout. Aucun endpoint RP de
réception ni Logout Token signé n'est annoncé. Les notifications protocolaires
proactives restent un travail distinct pour les clients disposant d'un serveur.

## Validation

Tests du contrôleur réel avec horloge simulée : périodicité, suspension,
reprise, requêtes lentes, événements regroupés, panne, fermeture et retrait
des écouteurs. Les autres tests Account couvrent l'expiration du jeton et les
réponses tardives.

Le scénario Chromium utilise les deux sites construits, les services réels et
l'horloge réelle. Une seconde page Identity ferme la session via son SDK et
les protections HTTP actives. Account reste visible et ne reçoit aucun événement
de focus/visibilité ; sa lecture périodique reçoit 401 puis retire le profil.

```bash
NVBES_IDENTITY_TEST_WEB_UI=1 NVBES_IDENTITY_TEST_ACCOUNT_WEB=1 NVBES_IDENTITY_TEST_ACCOUNT_REVALIDATION=1 pnpm nx run identity-service:test:https-browser-fixture
```

Construire les deux sites avant la fixture. Le mode utilise une base et des
comptes synthétiques isolés, puis nettoie ses processus et conteneurs. Aucun
déploiement ni trafic fournisseur réel.
