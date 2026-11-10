import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import icon from 'astro-icon';
import starlightLinksValidator from 'starlight-links-validator';

import { REPO_URL, REPO_BRANCH, SITE_URL, SITE_NAME, SITE_DESCRIPTION } from './src/config.ts';
import { headTags } from './src/lib/head.ts';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  site: SITE_URL,
  output: 'static',

  integrations: [
    icon({ include: { tabler: ['arrow-right', 'brand-github', 'copy', 'device-laptop', 'moon', 'sun'] } }),
    starlight({
      title: SITE_NAME,
      description: SITE_DESCRIPTION,
      logo: {
        light: '../brand/logos/typrest-logo-horizontal.svg',
        dark: '../brand/logos/typrest-logo-horizontal-inverse.svg',
        replacesTitle: true,
      },
      favicon: '/favicon.svg',
      // The landing layout renders the same tags; see src/lib/head.ts.
      head: headTags.map(({ tag, attrs }) => ({ tag, attrs })),
      // Order matters: brand/starlight/typrest.css is the only source of Starlight colors and
      // must load last.
      customCss: ['./src/styles/fonts.css', './src/styles/global.css', '../brand/starlight/typrest.css'],
      components: {
        Head: './src/components/starlight/Head.astro',
      },
      social: [{ icon: 'github', label: 'GitHub', href: REPO_URL }],
      editLink: { baseUrl: `${REPO_URL}/edit/${REPO_BRANCH}/website/` },
      // The landing layout provides the site-wide 404 page.
      disable404Route: true,
      sidebar: [
        { label: 'Start here', items: ['docs', 'docs/getting-started'] },
        { label: 'Specifications', items: [{ autogenerate: { directory: 'docs/specs' } }] },
        { label: 'Architecture', items: [{ autogenerate: { directory: 'docs/architecture' } }] },
        {
          label: 'Project',
          items: [
            { label: 'Examples', link: '/examples/' },
            { label: 'Roadmap', link: `${REPO_URL}/blob/${REPO_BRANCH}/ROADMAP.md`, attrs: { target: '_blank' } },
            { label: 'Contributing', link: `${REPO_URL}/blob/${REPO_BRANCH}/CONTRIBUTING.md`, attrs: { target: '_blank' } },
            { label: 'License', link: `${REPO_URL}/blob/${REPO_BRANCH}/LICENSE`, attrs: { target: '_blank' } },
          ],
        },
      ],
      plugins: [
        starlightLinksValidator({
          // Pages outside Starlight, rendered by src/pages/.
          exclude: ['/', '/examples/', '/examples/**'],
        }),
      ],
    }),
  ],

  vite: {
    plugins: [tailwindcss()],
    resolve: {
      alias: {
        '~': path.resolve(__dirname, './src'),
        '@brand': path.resolve(__dirname, '../brand'),
      },
    },
    server: {
      // Brand assets and the repository docs live outside website/.
      fs: { allow: ['.', '../brand'] },
    },
  },
});
