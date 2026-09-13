import { setupServer } from 'msw/node';
import { identityHandlers } from './handlers/identity.handlers';

/**
 * MSW Server for Node / Vitest testing environment.
 * Intercepts HTTP requests at network transport level without mocking application code.
 */
export const server = setupServer(...identityHandlers);
