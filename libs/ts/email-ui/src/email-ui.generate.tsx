import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { render } from 'react-email';
import { EmailActionTemplate } from './email-ui.action-template';
import { emailMarkers } from './email-ui.markers';
import { EmailShellTemplate } from './email-ui.shell-template';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const templatesRoot = resolve(packageRoot, '../../rust/email/templates');
const checkOnly = process.argv.includes('--check');

if (!checkOnly) {
  await mkdir(templatesRoot, { recursive: true });
}

const templates = [
  {
    path: resolve(templatesRoot, 'email.action.generated.html'),
    html: await render(<EmailActionTemplate {...emailMarkers} />),
  },
  {
    path: resolve(templatesRoot, 'email.shell.generated.html'),
    html: await render(
      <EmailShellTemplate
        preview={emailMarkers.preview}
        title={emailMarkers.title}
        year={emailMarkers.year}
      >
        {emailMarkers.content}
      </EmailShellTemplate>,
    ),
  },
];

for (const template of templates) {
  const expected = `${template.html.trim()}\n`;
  if (checkOnly) {
    const current = await readFile(template.path, 'utf8');
    if (current !== expected) {
      throw new Error(`Generated email template is stale: ${template.path}`);
    }
  } else {
    await writeFile(template.path, expected);
  }
}
