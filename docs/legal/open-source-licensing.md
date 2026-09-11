# Politique de licence open source

## Licence par défaut

Sauf indication contraire dans un manifeste de package ou un en-tête SPDX,
le code détenu par nvbes dans ce dépôt est distribué sous
`AGPL-3.0-only`. Le texte intégral se trouve dans [`LICENSE`](../../LICENSE).

L'AGPL impose notamment de proposer le code source correspondant aux personnes
qui interagissent avec une version modifiée du logiciel au travers d'un réseau.
Ce résumé n'est pas un avis juridique et ne remplace pas le texte de la licence.

## Exception déclarée

`libs/rust/identity-sdk-backend` est distribué sous `MIT`, conformément à son
`Cargo.toml`. Le texte MIT applicable se trouve dans
[`LICENSES/MIT.txt`](../../LICENSES/MIT.txt).

Les autres SDK et clients restent sous `AGPL-3.0-only` tant que leur manifeste
ne déclare pas explicitement une autre licence. Une modification de cette
politique exige une décision de licence explicite et une mise à jour coordonnée
des manifestes, paquets publiés et notices.

## Dépendances et contenus tiers

Les dépendances, données facultatives et fichiers tiers conservent leurs
propres licences. Leur présence n'implique pas qu'ils soient relicenciés sous
la licence nvbes. [`NOTICE.md`](../../NOTICE.md) décrit les jeux de données
réseau facultatifs et leurs contraintes de redistribution ; `cargo-deny`, OSV,
Trivy et le SBOM CycloneDX constituent les contrôles automatisés associés.

## Contributions

Une contribution acceptée est fournie sous la licence du composant modifié.
Le contributeur doit avoir le droit de soumettre son travail et signaler toute
portion de code ou donnée tierce avec sa provenance et sa licence.
