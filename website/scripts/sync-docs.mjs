// Generates Starlight pages from the repository's docs/ folder. Run before `dev` and `build`.
//
// - Publishes docs/specs/*.md and docs/architecture/*.md, except the specification template and
//   index files. Plans, ADRs, and guides are not published.
// - Moves the H1 into a frontmatter `title` and uses the Scope line as `description`; the
//   Status/Scope list is dropped from the body.
// - Rewrites links between published pages to site routes, and links to anything else in the
//   repository (ADRs, plans, source files) to GitHub.
// - Keeps REQ-* IDs and everything else as written. Never edits docs/.
//
// Output goes to gitignored folders under src/content/docs/docs/.
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync, existsSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { REPO_URL, REPO_BRANCH } from '../src/config.ts';

const websiteDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const repoDir = path.resolve(websiteDir, '..');
const outDir = path.join(websiteDir, 'src/content/docs/docs');

/** Published folders under docs/, each becoming /docs/<folder>/. */
const SECTIONS = ['specs', 'architecture'];
const EXCLUDED = new Set(['docs/specs/template.md', 'docs/specs/README.md', 'docs/architecture/README.md']);

/** Sidebar order inside each section; unlisted pages follow alphabetically. */
const ORDER = {
  specs: ['cli', 'typrest-toml', 'templates', 'schema-v1', 'http-api', 'diagnostics', 'logging', 'webui'],
  architecture: ['overview', 'template-lifecycle', 'compiler'],
};

const toPosix = (p) => p.split(path.sep).join('/');

const published = new Map();
for (const section of SECTIONS) {
  for (const file of readdirSync(path.join(repoDir, 'docs', section)).sort()) {
    const repoPath = `docs/${section}/${file}`;
    if (!file.endsWith('.md') || EXCLUDED.has(repoPath)) continue;
    published.set(repoPath, `/docs/${section}/${file.slice(0, -3)}/`);
  }
}

const githubUrl = (repoPath) => {
  const absolute = path.join(repoDir, repoPath);
  const kind = existsSync(absolute) && statSync(absolute).isDirectory() ? 'tree' : 'blob';
  return `${REPO_URL}/${kind}/${REPO_BRANCH}/${repoPath}`.replace(/\/$/, '');
};

/** Rewrites one link target found in `fromRepoPath`. */
const rewriteTarget = (target, fromRepoPath) => {
  if (/^[a-z][a-z0-9+.-]*:/i.test(target) || target.startsWith('#') || target.startsWith('/')) {
    return target;
  }
  const [pathPart, hash = ''] = target.split(/(?=#)/);
  const repoPath = toPosix(path.normalize(path.join(path.dirname(fromRepoPath), pathPart)));
  if (repoPath.startsWith('..')) {
    throw new Error(`${fromRepoPath}: link "${target}" leaves the repository`);
  }
  const route = published.get(repoPath);
  return route ? `${route}${hash}` : `${githubUrl(repoPath)}${hash}`;
};

const FENCE = /^\s*(```|~~~)/;
const INLINE_LINK = /(!?\[[^\]]*\]\()([^)\s]+)((?:\s+"[^"]*")?\))/g;
const REFERENCE_LINK = /^(\s*\[[^\]]+\]:\s*)(\S+)(.*)$/;

const rewriteLinks = (body, fromRepoPath) => {
  let inFence = false;
  return body
    .split('\n')
    .map((line) => {
      if (FENCE.test(line)) {
        inFence = !inFence;
        return line;
      }
      if (inFence) return line;
      const reference = REFERENCE_LINK.exec(line);
      if (reference) return `${reference[1]}${rewriteTarget(reference[2], fromRepoPath)}${reference[3]}`;
      return line.replace(INLINE_LINK, (_, open, target, close) => `${open}${rewriteTarget(target, fromRepoPath)}${close}`);
    })
    .join('\n');
};

/** Markdown inline syntax to plain text, for the description meta tag. */
const plainText = (markdown) =>
  markdown
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/`([^`]*)`/g, '$1')
    .replace(/\*\*([^*]*)\*\*/g, '$1')
    .replace(/\s+/g, ' ')
    .trim();

const yamlString = (text) => JSON.stringify(text);

/** Splits off the H1 and the leading `- **Status:**`/`- **Scope:**` list. */
const parse = (source, repoPath) => {
  const lines = source.split('\n');
  const h1 = lines.findIndex((line) => /^# /.test(line));
  if (h1 === -1) throw new Error(`${repoPath}: no H1 heading`);
  const title = plainText(lines[h1].slice(2));

  let index = h1 + 1;
  while (index < lines.length && lines[index].trim() === '') index++;
  const meta = [];
  while (index < lines.length && /^(- \*\*|\s{2,}\S)/.test(lines[index])) {
    if (lines[index].startsWith('- ')) meta.push(lines[index].slice(2));
    else meta[meta.length - 1] += ` ${lines[index].trim()}`;
    index++;
  }
  const scope = meta.find((item) => item.startsWith('**Scope:**'));
  const description = scope ? plainText(scope.slice('**Scope:**'.length)) : undefined;
  const body = lines.slice(index).join('\n').replace(/^\n+/, '');
  return { title, description, body };
};

for (const section of SECTIONS) rmSync(path.join(outDir, section), { recursive: true, force: true });

for (const [repoPath, route] of published) {
  const source = readFileSync(path.join(repoDir, repoPath), 'utf8');
  const { title, description, body } = parse(source, repoPath);
  const [section, slug] = route.split('/').filter(Boolean).slice(1);
  const order = ORDER[section]?.indexOf(slug) ?? -1;

  const frontmatter = [
    '---',
    `title: ${yamlString(title)}`,
    ...(description ? [`description: ${yamlString(description)}`] : []),
    `editUrl: ${yamlString(`${REPO_URL}/edit/${REPO_BRANCH}/${repoPath}`)}`,
    ...(order >= 0 ? ['sidebar:', `  order: ${order + 1}`] : []),
    '---',
    '',
    `<!-- Generated from ${repoPath} by website/scripts/sync-docs.mjs. Edit the source file. -->`,
    '',
  ].join('\n');

  const target = path.join(outDir, section, `${slug}.md`);
  mkdirSync(path.dirname(target), { recursive: true });
  writeFileSync(target, frontmatter + rewriteLinks(body, repoPath));
}

console.log(`sync-docs: generated ${published.size} pages from docs/`);
