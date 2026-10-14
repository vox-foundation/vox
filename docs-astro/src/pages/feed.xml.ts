import rss from '@astrojs/rss';
import { getCollection } from 'astro:content';
import type { APIContext } from 'astro';
import { getGitDates } from '../utils/git-dates.mjs';

export async function GET(context: APIContext) {
  const docs = await getCollection('docs');
  const gitDates = getGitDates();

  // Dates come from Git, not frontmatter. Keyed by repo-relative path (docs/src/...)
  const items = docs
    .map((doc) => {
      const cleanPath = doc.filePath
        ? doc.filePath.replace(/^src\/content\/docs\//, '')
        : doc.id.endsWith('.md') || doc.id.endsWith('.mdx')
        ? doc.id
        : `${doc.id}.md`;
      const key = `docs/src/${cleanPath}`;
      const date = gitDates.get(key) ?? gitDates.get(`docs/src/${doc.id}.md`) ?? gitDates.get(`docs/src/${doc.id}.mdx`);
      return { doc, date };
    })
    .filter((entry): entry is { doc: typeof entry.doc; date: string } => Boolean(entry.date))
    .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
    .slice(0, 30)
    .map(({ doc, date }) => ({
      title: doc.data.title,
      pubDate: new Date(date),
      link: `/${doc.id}/`,
      description: doc.data.description ?? '',
    }));

  // Fail the build rather than shipping an empty feed.
  if (items.length === 0) {
    throw new Error(
      `feed.xml produced 0 items from ${docs.length} docs and ` +
        `${gitDates.size} git dates. Refusing to publish an empty feed.`,
    );
  }

  return rss({
    title: 'Vox: The AI-Native Programming Language — Docs',
    description: 'Official documentation updates for the Vox language.',
    site: context.site!,
    items,
    customData: '<language>en-us</language>',
  });
}
