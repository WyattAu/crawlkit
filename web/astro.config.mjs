import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://wyattau.github.io',
  base: '/crawlkit',
  integrations: [
    starlight({
      title: 'crawlkit',
      logo: {
        src: './src/assets/logo.svg',
        replacesTitle: true,
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/WyattAu/crawlkit' },
      ],
      sidebar: [
        {
          label: 'Start Here',
          items: [
            { label: 'Overview', slug: 'index' },
            { label: 'Getting Started', slug: 'getting-started' },
            { label: 'Installation', slug: 'installation' },
          ],
        },
        {
          label: 'Core Concepts',
          items: [
            { label: 'Architecture', slug: 'architecture' },
            { label: 'Analyzers', slug: 'analyzers' },
            { label: 'Configuration', slug: 'configuration' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'CLI Reference', slug: 'cli-reference' },
            { label: 'API Reference', slug: 'api-reference' },
            { label: 'Export Formats', slug: 'export-formats' },
          ],
        },
        {
          label: 'Advanced',
          items: [
            { label: 'Custom Analyzers', slug: 'tutorials/custom-analyzers' },
            { label: 'CI Integration', slug: 'tutorials/ci-integration' },
            { label: 'Performance', slug: 'benchmarks' },
          ],
        },
      ],
      customCss: ['./src/styles/custom.css'],
    }),
  ],
});
