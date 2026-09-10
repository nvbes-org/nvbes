import { chromium } from 'playwright';
import { verifyHostedUi } from '../identity-service/tests/runtime-browser-hosted-ui.mjs';
import { verifyHostedMfa } from '../identity-service/tests/runtime-browser-hosted-mfa.mjs';

const origin = new URL(process.env.IDENTITY_WEB_TEST_CLIENT_ORIGIN ?? '');
if (
  origin.protocol !== 'https:' ||
  origin.hostname !== '127.0.0.1' ||
  !origin.port ||
  origin.username ||
  origin.password ||
  origin.pathname !== '/' ||
  origin.search ||
  origin.hash
) {
  throw new Error('Expected the client origin of an isolated HTTPS browser fixture.');
}
const browser = await chromium.launch();
try {
  console.log(
    JSON.stringify({
      browser: browser.version(),
      ...(await verifyHostedUi(
        browser,
        origin.origin,
        process.env.IDENTITY_WEB_TEST_SCREENSHOT_PREFIX,
      )),
      ...(await verifyHostedMfa(browser, origin.origin)),
    }),
  );
} finally {
  await browser.close();
}
