/** Normalize rendered React Email HTML for deterministic on-disk templates. */
export function normalizeGeneratedHtml(html: string): string {
  return `${html.trim()}\n`;
}
