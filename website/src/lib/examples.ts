// The examples gallery, built from the table in examples/README.md so that file stays the only
// index of examples (AGENTS.md). Rendered previews come from scripts/render-website-examples.sh,
// which writes src/assets/examples/manifest.json next to the images.
import type { ImageMetadata } from 'astro';

import readme from '../../../examples/README.md?raw';
import manifest from '../assets/examples/manifest.json';
import { REPO_URL, REPO_BRANCH } from '../config';

export interface Rendering {
  /** Template ID, such as `letters/formal`. */
  template: string;
  /** Public URL of the rendered PDF. */
  pdf: string;
  /** First-page preview, optimized by Astro. */
  image: ImageMetadata;
  pages: number;
}

export interface Example {
  name: string;
  /** What the example shows, as HTML. */
  description: string;
  source: string;
  renderings: Rendering[];
}

interface ManifestEntry {
  template: string;
  pdf: string;
  image: string;
  pages: number;
}

const ROW = /^\|\s*\[`([^`]+)`\]\([^)]*\)\s*\|\s*(.+?)\s*\|\s*$/;

const images = import.meta.glob<ImageMetadata>('../assets/examples/**/*.png', {
  eager: true,
  import: 'default',
});

const escapeHtml = (text: string) =>
  text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** Inline Markdown used in the table: code spans only. */
const inlineMarkdown = (text: string) => escapeHtml(text).replace(/`([^`]+)`/g, '<code>$1</code>');

const renderingsFor = (name: string): Rendering[] =>
  ((manifest as Record<string, ManifestEntry[]>)[name] ?? []).flatMap((entry) => {
    const image = images[`../assets/examples/${entry.image}`];
    if (!image) return [];
    return [{ template: entry.template, pdf: `/examples/${entry.pdf}`, image, pages: entry.pages }];
  });

export const examples: Example[] = readme
  .split('\n')
  .map((line) => ROW.exec(line))
  .filter((match): match is RegExpExecArray => match !== null)
  .map(([, name, description]) => ({
    name,
    description: inlineMarkdown(description),
    source: `${REPO_URL}/tree/${REPO_BRANCH}/examples/${name}`,
    renderings: renderingsFor(name),
  }));

if (examples.length === 0) {
  throw new Error('No examples found in examples/README.md; has the table format changed?');
}
