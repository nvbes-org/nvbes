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
