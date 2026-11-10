// robots.txt follows SITE_INDEXABLE, like the noindex meta tag (src/components/Robots.astro).
import type { APIRoute } from 'astro';
import { SITE_INDEXABLE } from '~/config';

export const GET: APIRoute = ({ site }) => {
  const body = SITE_INDEXABLE
    ? `User-agent: *\nAllow: /\n\nSitemap: ${new URL('sitemap-index.xml', site)}\n`
    : 'User-agent: *\nDisallow: /\n';
  return new Response(body, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
};
