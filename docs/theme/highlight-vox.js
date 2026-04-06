/* theme/highlight-vox.js */
/* Registered in book.toml as [output.html] additional-js = ["theme/highlight-vox.js"] */

hljs.registerLanguage('vox', function(hljs) {
  var KEYWORDS = {
    keyword: 'fn let mut if else match for in to ret import type pub with on actor workflow spawn http',
    literal: 'true false Unit',
    built_in: 'list map int float str bool Option Result Some None Ok Err Id'
  };

  return {
    name: 'Vox',
    aliases: ['vox'],
    keywords: KEYWORDS,
    contains: [
      hljs.HASH_COMMENT_MODE,
      hljs.QUOTE_STRING_MODE,
      hljs.C_NUMBER_MODE,
      {
        className: 'meta',
        begin: '@[a-z.]+'
      },
      {
        className: 'function',
        beginKeywords: 'fn',
        end: '(?=\\{)',
        contains: [
          hljs.UNDERSCORE_TITLE_MODE,
          {
            className: 'params',
            begin: '\\(',
            end: '\\)'
          }
        ]
      },
      {
         className: 'class',
         beginKeywords: 'type actor workflow',
         end: '(?=\\{)',
         contains: [hljs.UNDERSCORE_TITLE_MODE]
      }
    ]
  };
});

hljs.initHighlightingOnLoad();
