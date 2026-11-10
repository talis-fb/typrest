// Icons, manifest, theme color, and social card tags shared by every page. The landing layout
// renders them directly and Starlight receives them through its `head` config, so both kinds of
// page stay identical.
import { SITE_URL } from '../config.ts';

export interface HeadTag {
  tag: 'link' | 'meta';
  attrs: Record<string, string>;
}

export const SOCIAL_CARD_URL = `${SITE_URL}/social-card.png`;

export const headTags: HeadTag[] = [
  { tag: 'link', attrs: { rel: 'icon', href: '/favicon.ico', sizes: '48x48' } },
  { tag: 'link', attrs: { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' } },
  { tag: 'link', attrs: { rel: 'manifest', href: '/site.webmanifest' } },
  { tag: 'meta', attrs: { name: 'theme-color', content: '#F9F6F0', media: '(prefers-color-scheme: light)' } },
  { tag: 'meta', attrs: { name: 'theme-color', content: '#1A1714', media: '(prefers-color-scheme: dark)' } },
  { tag: 'meta', attrs: { property: 'og:image', content: SOCIAL_CARD_URL } },
  { tag: 'meta', attrs: { property: 'og:image:width', content: '1280' } },
  { tag: 'meta', attrs: { property: 'og:image:height', content: '640' } },
  { tag: 'meta', attrs: { name: 'twitter:image', content: SOCIAL_CARD_URL } },
];
