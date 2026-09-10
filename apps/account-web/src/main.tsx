import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { App } from './app';
import { AccountController } from './account.controller';
import { accountGateway } from './account.gateway';
import './styles.css';

const root = document.getElementById('root');
if (!root) throw new Error('Missing application root');
const controller = new AccountController(accountGateway(location.origin), (url) =>
  location.assign(url),
);
const callback = location.pathname === '/oauth/callback' ? new URL(location.href) : undefined;
// Remove authorization codes before rendering or making any network request.
if (callback) history.replaceState(null, '', '/');
window.addEventListener('pagehide', () => flushSync(() => controller.dispose()), { once: true });
void controller.start(callback);
window.addEventListener('focus', () => {
  if (document.visibilityState === 'visible') void controller.revalidate();
});
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') void controller.revalidate();
});
createRoot(root).render(<App controller={controller} />);
