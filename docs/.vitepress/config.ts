import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'Titrate',
  description: 'The Titrate Programming Language',
  themeConfig: {
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Reference', link: '/reference/lexer' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Introduction', link: '/guide/getting-started' },
            { text: 'Variables', link: '/guide/variables' },
            { text: 'Functions', link: '/guide/functions' },
            { text: 'Classes', link: '/guide/classes' },
            { text: 'Enums', link: '/guide/enums' },
            { text: 'Control Flow', link: '/guide/control-flow' },
            { text: 'Pattern Matching', link: '/guide/pattern-matching' },
            { text: 'Error Handling', link: '/guide/error-handling' },
            { text: 'Ownership', link: '/guide/ownership' },
            { text: 'Standard Library', link: '/guide/stdlib' },
          ],
        },
      ],
      '/reference/': [
        {
          text: 'Language Reference',
          items: [
            { text: 'Lexer Tokens', link: '/reference/lexer' },
            { text: 'Grammar', link: '/reference/grammar' },
            { text: 'Types', link: '/reference/types' },
            { text: 'Memory Model', link: '/reference/memory-model' },
          ],
        },
      ],
    },
  },
});
