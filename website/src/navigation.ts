import { REPO_URL, repoFile } from './config';

export interface NavLink {
  text: string;
  href: string;
}

export const headerLinks: NavLink[] = [
  { text: 'Docs', href: '/docs/' },
  { text: 'Getting started', href: '/docs/getting-started/' },
  { text: 'Examples', href: '/examples/' },
];

export const footerLinks: { title: string; links: NavLink[] }[] = [
  {
    title: 'Product',
    links: [
      { text: 'Getting started', href: '/docs/getting-started/' },
      { text: 'Examples', href: '/examples/' },
      { text: 'Documentation', href: '/docs/' },
    ],
  },
  {
    title: 'Project',
    links: [
      { text: 'GitHub', href: REPO_URL },
      { text: 'Roadmap', href: repoFile('ROADMAP.md') },
      { text: 'Contributing', href: repoFile('CONTRIBUTING.md') },
      { text: 'License', href: repoFile('LICENSE') },
    ],
  },
];
