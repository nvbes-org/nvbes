# Micro-tests et isolation

`pnpm test:unit` appelle `rust-workspace:micro-test`. Les dependances doivent
etre installees avant l'execution. Aucun `.env`, service local, installation
de paquet ou base de donnees n'est prepare par cette cible.

La compilation Cargo est une phase distincte. Le runner enumere ensuite les
tests `lib` et `bin` de chaque manifeste Cargo du catalogue V1, dont Billing.
Tous sont inclus par defaut. Les tests de transport et stockage sont classes
explicitement dans `tools/rust-workspace/micro-test.components.json` avec leur
raison. Une classification obsolete fait echouer le gate. Les tests classes
restent executes par `cargo test` et les lanes CI habituelles ; les tests SQLx
actives par `database-tests` restent dans les cibles `test:database` existantes.

## Isolation effective

Chaque executable de test tourne sous Seatbelt sur macOS ou seccomp sur Linux
(ARM64/x86-64), avec refus du reseau, loopback compris. Le sandbox Linux permet
l'IPC Unix necessaire au runtime, mais tue un processus ouvrant un socket d'une
autre famille. Le sandbox se propage aux enfants. Le gate echoue si l'isolation
est indisponible. Des controles positifs et negatifs IPv4/IPv6 sont executes
avant les tests ; `rust-workspace:test:harness` verifie aussi l'heritage aux enfants.

L'environnement des tests est restreint, le fuseau est UTC et les tests Rust
utilisent un seul thread pour les fixtures qui modifient l'environnement global.
Chaque processus est borne a 120 secondes. Le temps d'execution est affiche
separement de la compilation ; le chargement initial des binaires sur macOS
peut dominer ce temps. Les tests TypeScript email et le contrat SDK generes
tournent sous la meme protection reseau, sans retries.

Le runner appelle directement le Vitest installe avec Vite+, sans le lanceur
`vp` susceptible de provisionner un runtime via npm. Le harnais teste ce
demarrage sans reseau avant toute compilation Rust, y compris en CI Linux.

Les tests d'age utilisent une date explicite, notamment la veille et le jour
du treizieme anniversaire et un changement de date regional. La fonction
publique conserve l'horloge de production ; seul le calcul pur recoit la date.

## Proprietes generatives

Six proprietes `proptest` executent chacune 2 048 cas avec la graine 20260912 :

- somme bornee des scores, correspondance des raisons et determinisme ;
- monotonie pour les regles a contributions positives ;
- classification a chaque seuil, y compris les seuils egaux ;
- conservation de l'evaluation apres serialisation JSON ;
- projection independante de l'ordre et respect des fenetres temporelles ;
- terminaison et bornes du budget de retry.

Le shrinking et la persistance des contre-exemples de proptest restent actifs.
Conserver les fichiers de regression produits apres un echec et ajouter un cas
nomme si le contre-exemple revele un contrat metier manquant. Les proprietes
tournent aussi avec `cargo test`, donc dans la campagne de mutation.

## Mutation

Le lanceur transmet des chemins de manifeste absolus a cargo-mutants, y compris
pour le workspace racine, et conserve les lockfiles avec `--locked`. La
concurrence est bornee entre 1 et 4 workers (2 par defaut). Les tests PostgreSQL
necessitent une base locale jetable explicite via
`NVBES_SECURITY_TEST_DATABASE_URL` ; les tests de composant ne sont pas exclus
de cette campagne. Aucun resultat partiel ne vaut validation des seuils.

`pnpm test:rust:mutation` mesure toutes les crates de production du catalogue V1,
sur chaque workspace Cargo, avec un seuil minimal de 90%. Utiliser
`pnpm exec nx run rust-workspace:mutation -- --list` pour inspecter le perimetre.
Les rapports ont un repertoire unique et sont verifies avec le meme lecteur
strict que la preuve V1 : baseline reussie, phases coherentes, compte exhaustif,
sources connues, version epinglee. Les timeouts comptent comme non detectes.
Une crate sans mutant viable fait echouer la mesure.

`pnpm test:rust:mutation:baseline` conserve le diagnostic historique Audit/Email.
Ses seuils inferieurs et exclusions ne constituent pas une validation V1.
Stryker reste execute par `test-summary:mutation:typescript` pour `email-ui`.
Ces campagnes longues sont distinctes des micro-tests et n'ajoutent aucun service
ni cout recurrent. Un resultat local ne remplace pas une preuve CI authentifiee
sur le SHA final ; aucun score courant n'est deduit d'une ancienne baseline.

## Cache de compilation en CI

Les PR utilisent le backend GitHub natif de sccache, avec les jetons temporaires
du runner et la portee du ref de merge de la PR. Les runs suivants de cette PR
peuvent reutiliser ses objets compiles ; `main` ne restaure jamais ce backend et
conserve le stockage Scaleway protege. Une PR ne publie pas dans ce dernier.
Sans jeton GitHub de runtime, le repli est le disque ephemere, annonce dans les logs.

Les cles sccache distinguent les sources, le compilateur et les options, donc
les objets instrumentes pour la couverture ne remplacent pas ceux des tests
ordinaires. Les tests et les rapports restent executes/regeneres a chaque run.
Le nettoyage llvm-cov est conserve pour eviter des mesures perimees. Les etapes
de compilation et de link non prises en charge par sccache restent necessaires.

Le cache est active avant l'installation de cargo-llvm-cov. Le resume de job
affiche les hits, misses et erreurs de lecture/ecriture : un second run de la
meme PR est necessaire pour mesurer le gain, pas seulement constater un build
vert. Aucun serveur ni quota payant n'est ajoute ; les entrees restent soumises
au quota et a l'eviction du cache Actions du depot, sans changement de facturation.

References : [backend GHA sccache](https://github.com/mozilla/sccache/blob/main/docs/GHA.md)
et [portees du cache GitHub](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching).
