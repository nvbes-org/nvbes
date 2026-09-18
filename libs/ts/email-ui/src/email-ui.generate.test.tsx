import { afterEach, describe, expect, it, vi } from 'vitest';

const fs = vi.hoisted(() => ({ mkdir: vi.fn(), readFile: vi.fn(), writeFile: vi.fn() }));
vi.mock('node:fs/promises', () => fs);

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
  vi.resetModules();
});

describe('email template generation', () => {
  async function generate(args: string[]) {
    vi.stubGlobal('process', { ...process, argv: ['node', 'email-ui.generate.tsx', ...args] });
    await import('./email-ui.generate');
  }

  it('writes deterministic action and shell templates with every substitution marker', async () => {
    await generate([]);
    expect(fs.mkdir).toHaveBeenCalledWith(
      expect.stringMatching(/\/libs\/rust\/email\/templates$/u),
      { recursive: true },
    );
    expect(fs.readFile).not.toHaveBeenCalled();
    expect(fs.writeFile).toHaveBeenCalledTimes(2);
    for (const [file, html] of fs.writeFile.mock.calls) {
      expect(file).toMatch(
        /\/libs\/rust\/email\/templates\/email\.(action|shell)\.generated\.html$/u,
      );
      expect(html).toMatch(/\n$/u);
      // Reviewed HTML fixtures protect the email rendering and Outlook compatibility contract.
      expect(html).toMatchSnapshot(
        file.endsWith('email.action.generated.html') ? 'action' : 'shell',
      );
    }
    expect(fs.writeFile.mock.calls[0][1]).toContain('__NVBES_ACTION_URL__');
    expect(fs.writeFile.mock.calls[1][1]).toContain('__NVBES_CONTENT__');
  });

  it('checks both existing templates without writing', async () => {
    await generate([]);
    const templates = new Map(fs.writeFile.mock.calls.map(([file, html]) => [file, html]));
    vi.resetModules();
    fs.mkdir.mockClear();
    fs.writeFile.mockClear();
    fs.readFile.mockImplementation(async (file) => templates.get(file));

    await generate(['--check']);

    expect(fs.readFile).toHaveBeenCalledTimes(2);
    for (const [file, encoding] of fs.readFile.mock.calls) {
      expect(templates.has(file)).toBe(true);
      expect(encoding).toBe('utf8');
    }
    expect(fs.mkdir).not.toHaveBeenCalled();
    expect(fs.writeFile).not.toHaveBeenCalled();
  });

  it('refuses stale generated content without overwriting it', async () => {
    fs.readFile.mockResolvedValue('stale');
    await expect(generate(['--check'])).rejects.toThrow(
      /Generated email template is stale: .*email.action.generated.html/u,
    );
    expect(fs.writeFile).not.toHaveBeenCalled();
    expect(fs.mkdir).not.toHaveBeenCalled();
  });

  it('propagates missing files and write failures', async () => {
    fs.readFile.mockRejectedValue(new Error('missing template'));
    await expect(generate(['--check'])).rejects.toThrow('missing template');
    vi.resetModules();
    fs.writeFile.mockRejectedValue(new Error('read-only directory'));
    await expect(generate([])).rejects.toThrow('read-only directory');
  });
});
