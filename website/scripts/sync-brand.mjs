// Copies the brand kit's public files (icons and social card) from brand/ into public/. Run before
// `dev` and `build`; the copies are gitignored, so brand/ stays the only source.
import { copyFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const websiteDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const brandDir = path.resolve(websiteDir, '../brand');
const publicDir = path.join(websiteDir, 'public');

/** Published name in public/ → source under brand/. Keep in sync with website/.gitignore. */
export const BRAND_FILES = {
  'favicon.svg': 'favicon/favicon.svg',
  'favicon.ico': 'favicon/favicon.ico',
  'apple-touch-icon.png': 'favicon/apple-touch-icon.png',
  'icon-192.png': 'favicon/icon-192.png',
  'icon-512.png': 'favicon/icon-512.png',
  'social-card.png': 'social/social-card.png',
};

mkdirSync(publicDir, { recursive: true });
for (const [target, source] of Object.entries(BRAND_FILES)) {
  copyFileSync(path.join(brandDir, source), path.join(publicDir, target));
}

console.log(`sync-brand: copied ${Object.keys(BRAND_FILES).length} files from brand/`);
