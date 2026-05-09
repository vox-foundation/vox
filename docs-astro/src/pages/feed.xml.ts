import rss from '@astrojs/rss';
import { getCollection } from 'astro:content';
import type { APIContext } from 'astro';

export async function GET(context: APIContext) {
  const docs = await getCollection('docs');

  const items = docs
    .filter(doc => doc.data.last_updated)
    .sort((a, b) => {
      const da = new Date(a.data.last_updated!).getTime();
      const db = new Date(b.data.last_updated!).getTime();
      return db - da;
    })
    .slice(0, 30)
    .map(doc => ({
      title: doc.data.title,
      pubDate: new Date(doc.data.last_updated!),
      link: `/${doc.id}/`,
      description: doc.data.description ?? '',
    }));

  return rss({
    title: 'Vox: The AI-Native Programming Language — Docs',
    description: 'Official documentation updates for the Vox language.',
    site: context.site!,
    items,
    customData: '<language>en-us</language>',
  });
}
