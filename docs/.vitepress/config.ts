import { defineConfig } from 'vitepress';
import titrateLang from './titrate-lang.js';

export default defineConfig({
  title: 'Titrate',
  description: 'A systems programming language with precise types, ownership semantics, and expressive syntax',
  base: '/titrate/',
  cleanUrls: true,

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/titrate/favicon.svg' }],
    ['meta', { name: 'theme-color', content: '#e94560' }],
    ['meta', { name: 'og:type', content: 'website' }],
    ['meta', { name: 'og:title', content: 'Titrate — A Systems Programming Language' }],
    ['meta', { name: 'og:description', content: 'Precise. Safe. Expressive. A systems programming language with generics, ownership, and scientific computing built in.' }],
    ['meta', { name: 'og:image', content: '/titrate/og-image.png' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
  ],

  sitemap: {
    hostname: 'https://richie-rich90454.github.io/titrate/',
  },

  lastUpdated: true,

  markdown: {
    theme: {
      light: 'github-light',
      dark: 'github-dark',
    },
    languages: [titrateLang],
    lineNumbers: true,
    anchor: {
      slugify: (str: string) => str.toLowerCase().replace(/\s+/g, '-').replace(/[^\w-]/g, ''),
    },
  },

  themeConfig: {
    logo: '/favicon.svg',

    nav: [
      { text: 'Guide', link: '/guide/getting-started', activeMatch: '/guide/' },
      {
        text: 'Reference',
        items: [
          { text: 'Lexer Tokens', link: '/reference/lexer' },
          { text: 'Grammar', link: '/reference/grammar' },
          { text: 'Types', link: '/reference/types' },
          { text: 'Memory Model', link: '/reference/memory-model' },
        ],
      },
      { text: 'Stdlib', link: '/stdlib/lang', activeMatch: '/stdlib/' },
      {
        text: 'Community',
        items: [
          { text: 'Contributing', link: '/guide/contributing' },
          { text: 'FAQ', link: '/guide/faq' },
          { text: 'GitHub', link: 'https://github.com/richie-rich90454/titrate' },
        ],
      },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          collapsed: false,
          items: [
            { text: 'Introduction', link: '/guide/getting-started' },
            { text: 'FAQ', link: '/guide/faq' },
          ],
        },
        {
          text: 'Basics',
          collapsed: false,
          items: [
            { text: 'Variables', link: '/guide/variables' },
            { text: 'Functions', link: '/guide/functions' },
            { text: 'Control Flow', link: '/guide/control-flow' },
            { text: 'Strings & Literals', link: '/guide/raw-strings' },
          ],
        },
        {
          text: 'Types & Data Structures',
          collapsed: false,
          items: [
            { text: 'Classes', link: '/guide/classes' },
            { text: 'Interfaces', link: '/guide/interfaces' },
            { text: 'Enums', link: '/guide/enums' },
            { text: 'Tuples', link: '/guide/tuples' },
            { text: 'Generics', link: '/guide/generics' },
          ],
        },
        {
          text: 'Advanced',
          collapsed: true,
          items: [
            { text: 'Pattern Matching', link: '/guide/pattern-matching' },
            { text: 'Error Handling', link: '/guide/error-handling' },
            { text: 'Closures', link: '/guide/closures' },
            { text: 'Operator Overloading', link: '/guide/operator-overloading' },
            { text: 'Iterators', link: '/guide/iterators' },
            { text: 'Ranges', link: '/guide/ranges' },
            { text: 'Ownership', link: '/guide/ownership' },
          ],
        },
        {
          text: 'Modules & I/O',
          collapsed: true,
          items: [
            { text: 'Modules', link: '/guide/modules' },
            { text: 'File I/O', link: '/guide/file-io' },
          ],
        },
        {
          text: 'Ecosystem',
          collapsed: true,
          items: [
            { text: 'Scientific Computing', link: '/guide/scientific-computing' },
            { text: 'Bioinformatics', link: '/guide/bio-guide' },
            { text: 'Physics Simulation', link: '/guide/physics-guide' },
            { text: 'Machine Learning', link: '/guide/ml-guide' },
            { text: '3D Graphics & Games', link: '/guide/3d-graphics-guide' },
            { text: 'HFT Development', link: '/guide/hft-guide' },
            { text: 'Scientific Simulation', link: '/guide/simulation-guide' },
            { text: 'Optimizations', link: '/guide/optimizations' },
            { text: 'Syntax Sugar', link: '/guide/syntax-sugar' },
            { text: 'Standard Library', link: '/guide/stdlib' },
            { text: 'Build Tool', link: '/guide/build-tool' },
            { text: 'Cookbook', link: '/guide/cookbook' },
          ],
        },
        {
          text: 'Internals',
          collapsed: true,
          items: [
            { text: 'Compiler Architecture', link: '/guide/architecture' },
            { text: 'Contributing', link: '/guide/contributing' },
          ],
        },
        {
          text: 'Migration Guides',
          collapsed: true,
          items: [
            { text: 'From C/C++', link: '/guide/migration-from-c' },
            { text: 'From ECMAScript/TypeScript', link: '/guide/migration-from-ecmascript' },
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
      '/stdlib/': [
        {
          text: 'Core',
          collapsed: false,
          items: [
            { text: 'lang', link: '/stdlib/lang' },
            { text: 'operator', link: '/stdlib/operator' },
            { text: 'optional-variant', link: '/stdlib/optional-variant' },
            { text: 'numeric-limits', link: '/stdlib/numeric-limits' },
            { text: 'stringview', link: '/stdlib/stringview' },
            { text: 'traceback', link: '/stdlib/traceback' },
            { text: 'enum', link: '/stdlib/enum' },
            { text: 'abc', link: '/stdlib/abc' },
          ],
        },
        {
          text: 'Collections',
          collapsed: false,
          items: [
            { text: 'collections', link: '/stdlib/collections' },
            { text: 'array', link: '/stdlib/array' },
            { text: 'hashset', link: '/stdlib/hashset' },
            { text: 'heapq', link: '/stdlib/heapq' },
            { text: 'bisect', link: '/stdlib/bisect' },
            { text: 'itertools', link: '/stdlib/itertools' },
            { text: 'dataclass', link: '/stdlib/dataclass' },
            { text: 'pair', link: '/stdlib/pair' },
            { text: 'span', link: '/stdlib/span' },
            { text: 'forwardlist', link: '/stdlib/forwardlist' },
            { text: 'treemap', link: '/stdlib/treemap' },
            { text: 'treeset', link: '/stdlib/treeset' },
            { text: 'ringdeque', link: '/stdlib/ringdeque' },
            { text: 'chainmap', link: '/stdlib/chainmap' },
            { text: 'unionfind', link: '/stdlib/unionfind' },
          ],
        },
        {
          text: 'I/O & File System',
          collapsed: true,
          items: [
            { text: 'io', link: '/stdlib/io' },
            { text: 'contextlib', link: '/stdlib/contextlib' },
            { text: 'stringio', link: '/stdlib/stringio' },
            { text: 'format', link: '/stdlib/format' },
            { text: 'mmap', link: '/stdlib/mmap' },
            { text: 'fileutils', link: '/stdlib/fileutils' },
            { text: 'glob', link: '/stdlib/glob' },
            { text: 'fnmatch', link: '/stdlib/fnmatch' },
          ],
        },
        {
          text: 'Text & Serialization',
          collapsed: true,
          items: [
            { text: 'text', link: '/stdlib/text' },
            { text: 'regex', link: '/stdlib/regex' },
            { text: 'serialization', link: '/stdlib/serialization' },
            { text: 'xml-advanced', link: '/stdlib/xml-advanced' },
            { text: 'json-advanced', link: '/stdlib/json-advanced' },
            { text: 'data-files', link: '/stdlib/data-files' },
            { text: 'pprint', link: '/stdlib/pprint' },
            { text: 'difflib', link: '/stdlib/difflib' },
            { text: 'shlex', link: '/stdlib/shlex' },
            { text: 'unicodedata', link: '/stdlib/unicodedata' },
            { text: 'toml', link: '/stdlib/toml' },
            { text: 'configparser', link: '/stdlib/configparser' },
          ],
        },
        {
          text: 'Math & Science',
          collapsed: true,
          items: [
            { text: 'math', link: '/stdlib/math' },
            { text: 'special', link: '/stdlib/special' },
            { text: 'transform', link: '/stdlib/transform' },
            { text: 'geometry3d', link: '/stdlib/geometry3d' },
            { text: 'sparse-linalg', link: '/stdlib/sparse-linalg' },
            { text: 'optimization', link: '/stdlib/optimization' },
            { text: 'pde', link: '/stdlib/pde' },
            { text: 'bit', link: '/stdlib/bit' },
            { text: 'complex', link: '/stdlib/complex' },
            { text: 'fractions', link: '/stdlib/fractions' },
            { text: 'statistics', link: '/stdlib/statistics' },
            { text: 'chemistry', link: '/stdlib/chemistry' },
            { text: 'units', link: '/stdlib/units' },
            { text: 'bio', link: '/stdlib/bio' },
            { text: 'physics', link: '/stdlib/physics' },
            { text: 'materials', link: '/stdlib/materials' },
            { text: 'sigproc', link: '/stdlib/sigproc' },
            { text: 'image', link: '/stdlib/image' },
            { text: 'audio', link: '/stdlib/audio' },
            { text: 'ml', link: '/stdlib/ml' },
            { text: 'geom', link: '/stdlib/geom' },
            { text: 'nlp', link: '/stdlib/nlp' },
            { text: 'finance', link: '/stdlib/finance' },
          ],
        },
        {
          text: 'System & Networking',
          collapsed: true,
          items: [
            { text: 'system', link: '/stdlib/system' },
            { text: 'networking', link: '/stdlib/networking' },
            { text: 'concurrent', link: '/stdlib/concurrent' },
            { text: 'crypto', link: '/stdlib/crypto' },
            { text: 'crypto2', link: '/stdlib/crypto2' },
            { text: 'os', link: '/stdlib/os' },
            { text: 'platform', link: '/stdlib/platform' },
            { text: 'signal', link: '/stdlib/signal' },
            { text: 'socket', link: '/stdlib/socket' },
            { text: 'ipaddress', link: '/stdlib/ipaddress' },
            { text: 'threadpool', link: '/stdlib/threadpool' },
            { text: 'event', link: '/stdlib/event' },
            { text: 'threadlocal', link: '/stdlib/threadlocal' },
            { text: 'ssl', link: '/stdlib/ssl' },
            { text: 'sqlite', link: '/stdlib/sqlite' },
            { text: 'thread', link: '/stdlib/thread' },
          ],
        },
        {
          text: 'HFT & Simulation',
          collapsed: true,
          items: [
            { text: 'hft', link: '/stdlib/hft' },
            { text: 'sim', link: '/stdlib/sim' },
          ],
        },
        {
          text: 'Date & Time',
          collapsed: true,
          items: [
            { text: 'datetime', link: '/stdlib/datetime' },
            { text: 'zoneinfo', link: '/stdlib/zoneinfo' },
            { text: 'scheduler', link: '/stdlib/scheduler' },
          ],
        },
        {
          text: 'Utilities',
          collapsed: true,
          items: [
            { text: 'functools', link: '/stdlib/functools' },
            { text: 'logging', link: '/stdlib/logging' },
            { text: 'uuid', link: '/stdlib/uuid' },
            { text: 'argparse', link: '/stdlib/argparse' },
            { text: 'algorithms', link: '/stdlib/algorithms' },
            { text: 'binary', link: '/stdlib/binary' },
            { text: 'compression', link: '/stdlib/compression' },
            { text: 'gzip', link: '/stdlib/gzip' },
            { text: 'hmac', link: '/stdlib/hmac' },
          ],
        },
        {
          text: 'Internationalization',
          collapsed: true,
          items: [
            { text: 'locale', link: '/stdlib/locale' },
          ],
        },
        {
          text: 'Testing',
          collapsed: true,
          items: [
            { text: 'testing', link: '/stdlib/testing' },
            { text: 'assert', link: '/stdlib/assert' },
          ],
        },
      ],
    },

    editLink: {
      pattern: 'https://github.com/richie-rich90454/titrate/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },

    outline: {
      level: [2, 3],
      label: 'On This Page',
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/richie-rich90454/titrate' },
    ],

    search: {
      provider: 'local',
      options: {
        detailedView: true,
      },
    },

    footer: {
      message: 'Released under the <a href="https://github.com/richie-rich90454/titrate/blob/main/LICENSE">Apache-2.0 License</a>.',
      copyright: 'Copyright 2024-present Titrate Contributors',
    },

    docFooter: {
      prev: 'Previous',
      next: 'Next',
    },

    lastUpdated: {
      text: 'Updated at',
      formatOptions: {
        dateStyle: 'short',
        timeStyle: 'short',
      },
    },

    returnToTopLabel: 'Return to top',
    sidebarMenuLabel: 'Menu',
    darkModeSwitchLabel: 'Theme',
    lightModeSwitchTitle: 'Switch to light theme',
    darkModeSwitchTitle: 'Switch to dark theme',
  },
});
