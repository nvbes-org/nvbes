import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { renderToHtml } from '@json-render/react-email';
import { invitationTemplate } from '../src/templates/invitation.js';
import { passwordResetTemplate } from '../src/templates/password-reset.js';
import { verificationTemplate } from '../src/templates/verification.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DIST_DIR = path.resolve(__dirname, '../dist');

const templates = {
  verification: verificationTemplate,
  'password-reset': passwordResetTemplate,
  invitation: invitationTemplate,
};

async function build() {
  if (!fs.existsSync(DIST_DIR)) {
    fs.mkdirSync(DIST_DIR, { recursive: true });
  }

  for (const [name, spec] of Object.entries(templates)) {
    console.log(`Building template: ${name}...`);
    try {
      const html = await renderToHtml(spec as Parameters<typeof renderToHtml>[0]);
      const outPath = path.join(DIST_DIR, `${name}.html`);
      fs.writeFileSync(outPath, html);
      console.log(`  ✓ Saved to ${outPath}`);
    } catch (error) {
      console.error(`  ✗ Failed to build ${name}:`, error);
      process.exit(1);
    }
  }
}

build().catch((err) => {
  console.error(err);
  process.exit(1);
});
