import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://docs.nvbes.internal',
  vite: {
    build: {
      target: 'esnext',
    },
    optimizeDeps: {
      esbuildOptions: {
        target: 'esnext',
      },
    },
  },
  integrations: [
    starlight({
      title: 'nvbes Developer Docs',
      description: 'Documentation technique interne, architecture et catalogues auto-générés.',
      customCss: ['./src/styles/custom.css'],
      defaultLocale: 'root',
      locales: {
        root: {
          label: 'Français',
          lang: 'fr',
        },
      },
      components: {
        Head: './src/components/Head.astro',
      },
      sidebar: [
        {
          label: '🚀 Démarrage & Onboarding',
          autogenerate: { directory: 'onboarding' },
        },
        {
          label: '🏛️ Architecture & Domaines',
          autogenerate: { directory: 'architecture' },
        },
        {
          label: '📡 Référence des APIs',
          autogenerate: { directory: 'api' },
        },
        {
          label: '📜 Décisions d Architecture (ADR)',
          autogenerate: { directory: 'adr' },
        },
        {
          label: '⚡ Catalogues Déterministes (IR)',
          autogenerate: { directory: 'generated' },
        },
        {
          label: '🛠️ Guides & Runbooks',
          autogenerate: { directory: 'guides' },
        },
      ],
    }),
  ],
});
