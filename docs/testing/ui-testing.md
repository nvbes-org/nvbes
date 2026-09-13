# Stratégie UI Testing

> **Statut : aligné avec le socle V1.** Les composants Cloud/Drive historiques
> sont archivés. La référence canonique pour l'architecture et les spécifications
> détaillées des tests frontend est [frontend-experience-audit.md](frontend-experience-audit.md).

## Objectif

Définir comment verrouiller les composants, états critiques, accessibilité et comportements visuels essentiels du socle V1.

## Niveaux

### 1. Tests de composants (RTL + MSW v2)

Couvrir au minimum :

- rendu des composants métier critiques du socle V1 ;
- variantes de props et états (loading, error 4xx/5xx, empty, success) ;
- mock réseau strict au niveau HTTP via MSW v2 (interdiction de `vi.mock` sur les queries) ;
- callbacks, formulaires et interactions utilisateur principales ;
- accessibilité unitaire (`axe-core`) sur chaque composant interactif.

Composants prioritaires V1 :

- `RegisterForm` / `RegisterPage.field` ;
- `LoginPage` (étapes identifiant, mot de passe, MFA) ;
- `MfaMethodChoiceList` / `LoginPageMfaStep.form` ;
- `PasswordStrengthMeter` ;
- `CookieConsentSettings` / `TrackingConsentToggle` ;
- `VerifyEmailPage.card` / `VerifyEmailResultPage` ;
- Primitives Radix/shadcn (`Dialog`, `Sheet`, `Select`, `DropdownMenu`, `Button`, `Input`).

### 2. Tests de pages et parcours E2E (Playwright Multi-Navigateurs)

Parcours prioritaires :

- Inscription $\to$ vérification d'email $\to$ première connexion ;
- Enrôlement 2FA/MFA $\to$ challenge TOTP $\to$ protection anti-rejeu ;
- Contrat des cookies durcis (`__Host-`) $\to$ déconnexion et révocation de session ;
- Espace Compte : profil, sessions actives, export RGPD.

Matrice : Chromium, Firefox, WebKit (Safari), Mobile Chrome, Mobile Safari.

### 3. Vérifications visuelles (Playwright Native Screenshot)

Objectif :

- Détecter les régressions de layout, densité, hiérarchie, états critiques et responsive sans coût SaaS (conforme FinOps $\le 30$ € TTC/mois).

Priorités :

- Desktop principal ($1280 \times 800$) ;
- Tablette ($768 \times 1024$) ;
- Mobile ($375 \times 667$).
- Thèmes clair (`light`) et sombre (`dark`).

### 4. Accessibilité automatisée (Axe-core WCAG 2.1 & 2.2 AA)

- 0 violation sur tags `['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']` ;
- navigation et piégeage du focus clavier sur modales et dialogues ;
- conformité WCAG 2.2 : Target Size minimum $24 \times 24$ px ($44 \times 44$ px mobile), outline de focus $\ge 3:1$, pas d'authentification cognitive bloquante.

## Déclencheurs

- modification d'un composant de design system / primitive UI ;
- modification d'un composant ou formulaire métier critique ;
- ajout ou altération d'un état d'erreur ou d'un flux d'authentification ;
- changement de copy ou mise en page responsive ;
- préparation d'une release candidate (gate de pré-release).
