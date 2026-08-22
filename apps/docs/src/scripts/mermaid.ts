function getTheme(): 'default' | 'dark' {
  return document.documentElement.dataset.theme === 'light' ? 'default' : 'dark';
}

function extractMermaidCode(block: HTMLElement): string {
  // Extract lines from expressive-code line elements to preserve exact \n newlines
  const lines = block.querySelectorAll('.ec-line');
  if (lines.length > 0) {
    return Array.from(lines)
      .map((l) => l.textContent || '')
      .join('\n');
  }
  return block.innerText || block.textContent || '';
}

async function renderMermaidDiagrams() {
  const blocks = document.querySelectorAll(
    'pre[data-language="mermaid"], .language-mermaid, pre:has(code.language-mermaid)'
  );
  if (blocks.length === 0) return;

  try {
    const { default: mermaid } = await import('mermaid');
    const theme = getTheme();

    mermaid.initialize({
      startOnLoad: false,
      theme,
      securityLevel: 'loose',
      fontFamily: 'inherit',
    });

    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i] as HTMLElement;
      if (block.dataset.mermaidProcessed === 'true') continue;
      block.dataset.mermaidProcessed = 'true';

      const code = extractMermaidCode(block);
      if (!code.trim()) continue;

      const container = document.createElement('div');
      container.className = 'mermaid-card flex justify-center items-center overflow-x-auto my-4';
      const diagram = document.createElement('div');
      diagram.className = 'mermaid';
      diagram.textContent = code.trim();
      container.append(diagram);

      const targetToReplace = block.closest('.expressive-code') || block.closest('figure') || block;
      targetToReplace.parentElement?.replaceChild(container, targetToReplace);
    }

    await mermaid.run({ querySelector: '.mermaid' });
  } catch (err) {
    console.error('[Mermaid Error]:', err);
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', renderMermaidDiagrams);
} else {
  renderMermaidDiagrams();
}

document.addEventListener('astro:page-load', renderMermaidDiagrams);

const observer = new MutationObserver(() => {
  const cards = document.querySelectorAll('.mermaid-card');
  cards.forEach((c) => c.remove());
  renderMermaidDiagrams();
});

observer.observe(document.documentElement, {
  attributes: true,
  attributeFilter: ['data-theme'],
});
