# @nvbes/identity-sdk

SDK TypeScript pour intégrer nvbes Identity (OAuth2) dans n'importe quel produit.

## Installation

```bash
pnpm add @nvbes/identity-sdk
```

## Utilisation

### 1. Initialiser le SDK

```typescript
import { NvbesIdentity } from '@nvbes/identity-sdk';

const identity = new NvbesIdentity({
  clientId: 'votre-client-id',
  clientSecret: 'votre-client-secret', // Optionnel pour les apps publiques
  redirectUri: 'https://votre-app.com/callback',
  authorizationUrl: 'https://identity.nvbes.eu/oauth/authorize',
  tokenUrl: 'https://identity.nvbes.eu/oauth/token',
  userInfoUrl: 'https://identity.nvbes.eu/oauth/userinfo',
});
```

### 2. Rediriger vers la page de connexion

```typescript
function LoginButton() {
  const handleLogin = () => {
    const authUrl = identity.getAuthorizationUrl(
      'openid profile email',
      'random-state-string' // Anti-CSRF
    );
    window.location.href = authUrl;
  };

  return <button onClick={handleLogin}>Se connecter</button>;
}
```

### 3. Gérer le callback OAuth2

```typescript
// Dans votre page /callback
async function CallbackPage() {
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const code = params.get('code');

    if (code) {
      try {
        const token = await identity.exchangeCode(code);
        identity.saveToken(); // Conserve le token en mémoire uniquement
        // Rediriger vers l'app
        window.location.href = '/dashboard';
      } catch (error) {
        console.error('OAuth2 error:', error);
      }
    }
  }, []);

  return <div>Connexion en cours...</div>;
}
```

### 4. Récupérer les infos utilisateur

```typescript
async function Dashboard() {
  const [user, setUser] = useState(null);

  useEffect(() => {
    identity.loadToken(); // Pas de stockage persistant côté client
    const userInfo = await identity.getUserInfo();
    setUser(userInfo);
  }, []);

  if (!identity.isAuthenticated()) {
    return <div>Veuillez vous connecter</div>;
  }

  return (
    <div>
      <h1>Bienvenue {user.name}</h1>
      <p>Email: {user.email}</p>
      <p>Workspace: {user.workspaceId}</p>
    </div>
  );
}
```

## API

### `getAuthorizationUrl(scope?, state?)`

Génère l'URL d'autorisation pour le flux OAuth2.

### `exchangeCode(code)`

Échange un code d'autorisation contre un token d'accès.

### `refreshToken(refreshToken)`

Rafraîchit un access token avec le grant OAuth `refresh_token`. Les refresh tokens nvbes sont rotatifs et doivent être remplacés par la nouvelle valeur retournée.

### `getUserInfo()`

Récupère les informations de l'utilisateur connecté.

### `isAuthenticated()`

Vérifie si l'utilisateur est connecté.

### `logout()`

Déconnecte l'utilisateur (supprime le token local).

### `saveToken()`

Conserve le token uniquement en mémoire.

### `loadToken()`

Ne charge aucune persistance locale.

## Build

```bash
pnpm build
```

Génère les fichiers dans `dist/` (CJS + ESM + types).
