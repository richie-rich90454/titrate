import { defineConfig } from 'vitepress';
import titrateLang from './titrate-lang.js';

export default defineConfig({
  title: 'Titrate',
  description: 'A systems programming language',
  base: '/titrate/',
  cleanUrls: true,
  markdown: {
    theme: 'github-dark',
    languages: [titrateLang],
  },
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/titrate/favicon.svg' }],
  ],
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
            { text: 'Generics', link: '/guide/generics' },
            { text: 'Modules', link: '/guide/modules' },
            { text: 'Control Flow', link: '/guide/control-flow' },
            { text: 'Pattern Matching', link: '/guide/pattern-matching' },
            { text: 'Error Handling', link: '/guide/error-handling' },
            { text: 'Closures', link: '/guide/closures' },
            { text: 'Tuples', link: '/guide/tuples' },
            { text: 'Operator Overloading', link: '/guide/operator-overloading' },
            { text: 'Iterators', link: '/guide/iterators' },
            { text: 'Ranges', link: '/guide/ranges' },
            { text: 'Raw Strings and Literals', link: '/guide/raw-strings' },
            { text: 'File I/O', link: '/guide/file-io' },
            { text: 'Ownership', link: '/guide/ownership' },
            { text: 'Scientific Computing', link: '/guide/scientific-computing' },
            { text: 'Optimizations', link: '/guide/optimizations' },
            { text: 'Standard Library', link: '/guide/stdlib' },
            { text: 'Build Tool', link: '/guide/build-tool' },
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
    socialLinks: [
      { icon: 'github', link: 'https://github.com/richie-rich90454/titrate' },
    ],
    search: {
      provider: 'local',
    },
    footer: {
      message: 'Released under the Apache-2.0 License.',
    },
  },
});
