import { expect, type Locator, type Page, test } from '@playwright/test';
import {
  attachMonkeyRun,
  boundedInteger,
  collectDiagnostics,
  diagnosticMessage,
  expectHealthyDocument,
  type JournalEntry,
  pick,
  type Random,
  randomLocator,
  randomToken,
  xorshift,
} from './monkey.support';

const webOrigin = new URL(process.env.NVBES_WEB_BASE_URL ?? 'http://127.0.0.1:3001').origin;
const publicRoutes = ['/login', '/register', '/verify', '/verify-email', '/verify-result'] as const;
const viewports = [
  { width: 320, height: 568 },
  { width: 390, height: 844 },
  { width: 768, height: 1_024 },
  { width: 1_440, height: 900 },
] as const;

type MonkeyAction = {
  name: string;
  run: (page: Page, random: Random, iteration: number) => Promise<string>;
};

const monkeyActions: readonly MonkeyAction[] = [
  { name: 'navigate', run: navigate },
  { name: 'reload', run: reload },
  { name: 'fill-field', run: fillField },
  { name: 'clear-field', run: clearField },
  { name: 'toggle-checkbox', run: toggleCheckbox },
  { name: 'follow-auth-link', run: followAuthLink },
  { name: 'click-button', run: clickButton },
  { name: 'keyboard', run: useKeyboard },
  { name: 'resize', run: resizeViewport },
  { name: 'media-preferences', run: changeMediaPreferences },
  { name: 'scroll', run: scrollDocument },
  { name: 'clear-browser-state', run: clearBrowserState },
];

test.describe('@monkey bounded public auth exploration', () => {
  test('seeded actions preserve the hosted auth boundary and a healthy document', async ({
    page,
  }, testInfo) => {
    test.slow();

    const seed = boundedInteger(process.env.NVBES_MONKEY_SEED, 20_260_729, 1, 0xffff_ffff);
    const actionCount = boundedInteger(process.env.NVBES_MONKEY_ACTIONS, 120, 1, 500);
    const random = xorshift(seed);
    const diagnostics = collectDiagnostics(page, webOrigin);
    const journal: JournalEntry[] = [];

    try {
      await page.goto('/login', { waitUntil: 'domcontentloaded' });
      await expectHealthyDocument(page, seed, undefined, diagnostics);

      for (let iteration = 0; iteration < actionCount; iteration += 1) {
        const action = pick(monkeyActions, random);
        const urlBefore = page.url();
        let detail = 'action did not complete';

        try {
          detail = await action.run(page, random, iteration);
        } finally {
          journal.push({
            action: action.name,
            detail,
            iteration,
            urlAfter: page.url(),
            urlBefore,
          });
        }

        await expectHealthyDocument(page, seed, journal.at(-1), diagnostics);
      }

      expect(diagnostics.pageErrors, diagnosticMessage(seed, 'page errors')).toEqual([]);
      expect(diagnostics.consoleErrors, diagnosticMessage(seed, 'console errors')).toEqual([]);
      expect(diagnostics.serverErrors, diagnosticMessage(seed, 'HTTP 5xx responses')).toEqual([]);
      expect(diagnostics.failedRequests, diagnosticMessage(seed, 'failed requests')).toEqual([]);
    } finally {
      await attachMonkeyRun(testInfo, seed, actionCount, journal, diagnostics);
    }
  });
});

async function navigate(page: Page, random: Random, iteration: number): Promise<string> {
  const route = pick(publicRoutes, random);
  const query = new URLSearchParams({ monkey_iteration: String(iteration) });
  await page.goto(`${route}?${query}`, { waitUntil: 'domcontentloaded' });
  return route;
}

async function reload(page: Page): Promise<string> {
  await page.reload({ waitUntil: 'domcontentloaded' });
  return 'current document';
}

async function fillField(page: Page, random: Random, iteration: number): Promise<string> {
  const fields = editableFields(page);
  const field = await randomLocator(fields, random);
  if (!field) return 'no editable field';

  const type = (await field.getAttribute('type')) ?? 'text';
  const name = (await field.getAttribute('name')) ?? type;
  const value =
    type === 'email'
      ? pick(
          [
            `monkey-${iteration}-${randomToken(random)}@example.test`,
            `monkey+${iteration}@sub.example.test`,
            'adresse-invalide',
            '',
          ],
          random,
        )
      : type === 'password'
        ? pick(
            ['Court1!', `Monkey-${randomToken(random)}-Solide!42`, "apostrophe-'‑unicode-密碼"],
            random,
          )
        : pick(['', 'texte', "apostrophe-'", 'accents-éèà', '漢字'], random);

  await field.fill(value);
  return `${name} (${type}, ${value.length} chars)`;
}

async function clearField(page: Page, random: Random): Promise<string> {
  const field = await randomLocator(editableFields(page), random);
  if (!field) return 'no editable field';
  const name = (await field.getAttribute('name')) ?? 'unnamed';
  await field.clear();
  await field.blur();
  return name;
}

async function toggleCheckbox(page: Page, random: Random): Promise<string> {
  const checkbox = await randomLocator(page.getByRole('checkbox'), random);
  if (!checkbox || !(await checkbox.isEnabled())) return 'no enabled checkbox';
  const name = (await checkbox.getAttribute('aria-label')) ?? (await checkbox.getAttribute('id'));
  await checkbox.click();
  return name ?? 'unnamed checkbox';
}

async function followAuthLink(page: Page, random: Random): Promise<string> {
  const links = page.locator(
    'a[href^="/login"]:visible, a[href^="/register"]:visible, a[href^="/verify"]:visible',
  );
  const link = await randomLocator(links, random);
  if (!link) return 'no hosted auth link';
  const href = (await link.getAttribute('href')) ?? 'unknown';
  await link.click();
  return href;
}

async function clickButton(page: Page, random: Random): Promise<string> {
  const button = await randomLocator(page.locator('button:visible:not([disabled])'), random);
  if (!button) return 'no enabled button';
  const name = (await button.textContent())?.trim().replaceAll(/\s+/gu, ' ') || 'unnamed button';
  await button.click();
  return name.slice(0, 120);
}

async function useKeyboard(page: Page, random: Random): Promise<string> {
  const key = pick(['Tab', 'Shift+Tab', 'Enter', 'Escape', 'Home', 'End'] as const, random);
  await page.keyboard.press(key);
  return key;
}

async function resizeViewport(page: Page, random: Random): Promise<string> {
  const viewport = pick(viewports, random);
  await page.setViewportSize(viewport);
  return `${viewport.width}x${viewport.height}`;
}

async function changeMediaPreferences(page: Page, random: Random): Promise<string> {
  const colorScheme = pick(['light', 'dark', 'no-preference'] as const, random);
  const reducedMotion = pick(['reduce', 'no-preference'] as const, random);
  await page.emulateMedia({ colorScheme, reducedMotion });
  return `${colorScheme}/${reducedMotion}`;
}

async function scrollDocument(page: Page, random: Random): Promise<string> {
  const direction = random() < 0.5 ? -1 : 1;
  const distance = Math.floor(random() * 800) * direction;
  await page.mouse.wheel(0, distance);
  return `${distance}px`;
}

async function clearBrowserState(page: Page): Promise<string> {
  await page.evaluate(() => {
    globalThis.localStorage.clear();
    globalThis.sessionStorage.clear();
  });
  return 'localStorage/sessionStorage';
}

function editableFields(page: Page): Locator {
  return page.locator(
    'input:visible:not([disabled]):not([readonly]):not([type="checkbox"]):not([type="radio"]):not([type="file"]), textarea:visible:not([disabled]):not([readonly])',
  );
}
