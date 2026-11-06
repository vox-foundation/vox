import test from 'node:test';
import assert from 'node:assert/strict';
import { linksValidatorExclude } from '../../src/utils/links-validator-exclude.mjs';

const docPages = [
  { relPath: 'index.mdx', id: '' },
  { relPath: 'tutorials/tut-fullstack-walkthrough.mdx', id: 'tutorials/tut-fullstack-walkthrough' },
  { relPath: 'reference/installation.md', id: 'reference/installation' },
];
const MD = '/repo/docs-astro/src/content/docs/tutorials/tut-first-app.md';
const MDX = '/repo/docs/src/index.mdx';
const exclude = linksValidatorExclude(docPages);

const cases = [
  [true, MDX, '#try-it', 'link inside an .mdx page'],
  [true, MDX, '/reference/installation/#anything', 'any link inside an .mdx page'],
  [true, MD, '/tutorials/tut-fullstack-walkthrough/', 'link targeting an .mdx route'],
  [true, MD, '/', 'link targeting the .mdx home page'],
  [true, MD, '/#try-it', 'hash on the .mdx home page'],
  [true, MD, 'http://127.0.0.1:11434', 'documented local probe URL'],
  [false, MD, '/reference/installation/#no-such-heading', 'bad hash on a .md route stays validated'],
  [false, MD, '/reference/no-such-page/', 'missing page stays validated'],
  [false, MD, '#missing-heading', 'same-page hash on a .md page is not the .mdx home page'],
  [false, MD, 'http://127.0.0.1:8080/', 'other local links stay validated'],
];

for (const [want, file, link, why] of cases) {
  test(`${want ? 'excludes' : 'validates'}: ${link} from ${file.split('/').pop()} (${why})`, () => {
    assert.equal(exclude({ file, link }), want);
  });
}
