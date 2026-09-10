import { chromium } from 'playwright';
import { verifyHostedUi } from '../identity-service/tests/runtime-browser-hosted-ui.mjs';
import { verifyHostedMfa } from '../identity-service/tests/runtime-browser-hosted-mfa.mjs';
import { verifyHostedEnrollment } from '../identity-service/tests/runtime-browser-hosted-enrollment.mjs';

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
const suite = process.env.IDENTITY_WEB_TEST_SUITE ?? 'authentication';
if (!['authentication', 'enrollment-totp', 'enrollment-passkey'].includes(suite))
  throw new Error('Unknown Identity Web browser suite');
const browser = await chromium.launch();
try {
  if (suite !== 'authentication') {
    console.log(
      JSON.stringify({
        browser: browser.version(),
        ...(await verifyHostedEnrollment(
          browser,
          origin.origin,
          suite.slice('enrollment-'.length),
        )),
      }),
    );
  } else {
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
  }
} finally {
  await browser.close();
}
