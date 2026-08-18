# @nvbes/email-ui

Registry interne de composants React Email inspirés de la direction artistique shadcn des
interfaces Identity et Account.

Les composants web shadcn/Radix ne sont pas importés : ils dépendent du navigateur et de CSS que
les clients mail ne supportent pas. Ce package partage leurs tokens visuels et produit du HTML avec
des styles inline et des layouts compatibles Outlook.

`pnpm nx run email-ui:generate` précompile les templates dans `libs/rust/email/templates`. Le worker
Rust consomme ces artefacts sans dépendre d'un runtime Node en production. Le target
`generated:check` échoue si les artefacts ne correspondent plus aux composants React.
