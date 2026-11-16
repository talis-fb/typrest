// Site-wide settings. Changing the domain or repository is a one-line edit here; nothing else in
// the site hard-codes them.

/** Public origin of the site, without a trailing slash. */
export const SITE_URL = 'https://typrest.talison.dev';

/** GitHub repository, used for source links and links to unpublished documents. */
export const REPO_URL = 'https://github.com/talis-fb/typrest';

/** Branch that GitHub links point to. */
export const REPO_BRANCH = 'main';

/**
 * Cloudflare Web Analytics token. Public by design: it ships in every page. The beacon renders
 * only in production builds and only when this is set.
 */
export const ANALYTICS_TOKEN = '';

/**
 * Whether search engines may index the site. Keep `false` until the repository is public; it
 * controls the `noindex` meta tag and `robots.txt`.
 */
export const SITE_INDEXABLE = false;

export const SITE_NAME = 'Typrest';

export const SITE_DESCRIPTION =
  'Typrest is an HTTP service that validates JSON payloads, renders Typst templates, and returns PDF bytes.';

/** Release the install commands point at. Bump it when a new version is published. */
export const TYPREST_VERSION = '0.1.0';

/** Container image, tagged with the release's major.minor line. */
export const DOCKER_IMAGE = `ghcr.io/talis-fb/typrest:${TYPREST_VERSION.split('.').slice(0, 2).join('.')}`;

/** Name of the Linux release archive attached to the GitHub release. */
export const RELEASE_ARCHIVE = `typrest-v${TYPREST_VERSION}-x86_64-linux.tar.gz`;

/** Download URL of a release asset. */
export const releaseAsset = (file: string): string => `${REPO_URL}/releases/download/v${TYPREST_VERSION}/${file}`;

/** Link to a file in the repository on GitHub. */
export const repoFile = (path: string): string => `${REPO_URL}/blob/${REPO_BRANCH}/${path}`;
