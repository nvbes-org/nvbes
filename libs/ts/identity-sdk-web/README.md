# @nvbes/identity-sdk-web

SDK TypeScript pour intégrer nvbes Identity dans les applications web navigateur.

## Sécurité

- **Aucun token stocké en JavaScript** (évite XSS)
- Utilise uniquement des **cookies HttpOnly** gérés par le backend identity
- Supporte PKCE pour les flux OAuth2 publics
- Compatible avec les domaines/subdomains (`.nvbes.eu`)

## Installation

```bash
pnpm add @nvbes/identity-sdk-web
```

## Utilisation

### 1. Initialiser le SDK

```typescript
import { nvbesIdentityWeb } from '@nvbes/identity-sdk-web';

const identity = new nvbesIdentityWeb({
  baseUrl: 'https://identity.nvbes.eu',
  clientId: 'votre-client-id',
  redirectUri: 'https://app.nvbes.eu/callback',
  cookieDomain: '.nvbes.eu',
  secureCookies: true,
});
```

### 2. Login avec PKCE (recommandé)

```typescript
function LoginButton() {
  const handleLogin = async () => {
    // Redirige vers identity.nvbes.eu avec PKCE
    await identity.redirectToLogin({
      scope: 'openid profile email',
      usePKCE: true,
    });
  };

  return <button onClick={handleLogin}>Se connecter</button>;
}
```

### 3. Callback (page /callback)

```typescript
async function CallbackPage() {
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const code = params.get('code');

    if (code) {
      // Le backend doit échanger le code et setter le cookie
      await fetch('/api/callback', {
        method: 'POST',
        body: JSON.stringify({ code }),
      });
      window.location.href = '/dashboard';
    }
  }, []);

  return <div>Connexion en cours...</div>;
}
```

### 4. Vérifier l'authentification

```typescript
if (await identity.isAuthenticated()) {
  const user = await identity.getCurrentUser();
  console.log('Bienvenue', user.name);
}
```

## API

### `redirectToLogin(options?)`

Redirige vers la page de login identity (avec PKCE par défaut).

### `getCurrentUser()`

Récupère les infos utilisateur via le cookie de session.

### `logout()`

Déconnecte (supprime les cookies côté backend).

### `isAuthenticated()`

Vérifie si l'utilisateur a un cookie de session valide.

### Détection d'environnement

Le SDK expose un parseur sans dépendance pour normaliser le navigateur, le système et le type
d'appareil. Les détecteurs spécialisés (`detectBrowser`, `detectBrowserVersion`, `detectOS`,
`detectDevice` et `detectDeviceType`) sont également exportés.

```typescript
import { parseUserAgent } from '@nvbes/identity-sdk-web';

const environment = parseUserAgent(navigator.userAgent, {
  vendor: navigator.vendor,
});

// {
//   browser: 'Mobile Safari',
//   browserVersion: 17.5,
//   device: 'iPhone',
//   deviceType: 'Mobile',
//   os: 'iOS',
//   osVersion: '17.5.0',
// }
```

`collectDeviceProfile()` continue de produire uniquement des catégories grossières destinées à
la confiance d'appareil. Les noms et versions détaillés retournés par `parseUserAgent()` n'y sont
pas ajoutés automatiquement.
