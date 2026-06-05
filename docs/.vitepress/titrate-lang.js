export default {
  scopeName: 'source.titrate',
  name: 'titrate',
  patterns: [
    { include: '#comments' },
    { include: '#keywords' },
    { include: '#types' },
    { include: '#literals' },
    { include: '#operators' },
    { include: '#punctuation' },
    { include: '#identifiers' },
  ],
  repository: {
    comments: {
      patterns: [
        {
          name: 'comment.line.double-slash.titrate',
          match: '//.*$',
        },
        {
          name: 'comment.block.titrate',
          begin: '/\\*',
          end: '\\*/',
        },
      ],
    },
    keywords: {
      patterns: [
        {
          name: 'keyword.control.titrate',
          match: '\\b(if|else|while|for|return|break|continue|switch|case|default|in)\\b',
        },
        {
          name: 'keyword.declaration.titrate',
          match: '\\b(let|var|const|fn|class|interface|enum|extends|implements|import|module|type)\\b',
        },
        {
          name: 'keyword.operator.titrate',
          match: '\\b(as|is)\\b',
        },
        {
          name: 'keyword.other.titrate',
          match: '\\b(new|this|super|unsafe|region|public|private)\\b',
        },
        {
          name: 'constant.language.titrate',
          match: '\\b(true|false|null)\\b',
        },
        {
          name: 'constant.other.titrate',
          match: '\\b(Ok|Err|Result|Owned)\\b',
        },
      ],
    },
    types: {
      patterns: [
        {
          name: 'support.type.primitive.titrate',
          match: '\\b(void|bool|byte|short|int|long|vast|uvast|float|double|half|quad|char|string|size|u8|u16|u32|u64)\\b',
        },
      ],
    },
    literals: {
      patterns: [
        {
          name: 'string.quoted.double.titrate',
          begin: '"',
          end: '"',
          patterns: [
            { name: 'constant.character.escape.titrate', match: '\\\\[nt\\\\\"0]' },
          ],
        },
        {
          name: 'string.quoted.single.titrate',
          begin: "'",
          end: "'",
          patterns: [
            { name: 'constant.character.escape.titrate', match: '\\\\[nt\\\\'\'0]' },
          ],
        },
        {
          name: 'constant.numeric.titrate',
          match: '\\b(0[xX][0-9a-fA-F_]+|0[oO][0-7_]+|0[bB][01_]+|[0-9][0-9_]*)\\b',
        },
        {
          name: 'constant.numeric.float.titrate',
          match: '\\b[0-9][0-9_]*\\.[0-9][0-9_]*[hq]?\\b',
        },
      ],
    },
    operators: {
      patterns: [
        {
          name: 'keyword.operator.arithmetic.titrate',
          match: '[+\\-*/%]',
        },
        {
          name: 'keyword.operator.comparison.titrate',
          match: '(==|!=|<=?|>=?)',
        },
        {
          name: 'keyword.operator.logical.titrate',
          match: '(&&|\\|\\||!)',
        },
        {
          name: 'keyword.operator.bitwise.titrate',
          match: '(&|\\||\\^|~|<<|>>)',
        },
        {
          name: 'keyword.operator.assignment.titrate',
          match: '=',
        },
        {
          name: 'keyword.operator.other.titrate',
          match: '(\\?|->|=>|::|&mut)',
        },
      ],
    },
    punctuation: {
      patterns: [
        {
          name: 'punctuation.titrate',
          match: '[{}()\\[\\].,;:]',
        },
      ],
    },
    identifiers: {
      patterns: [
        {
          name: 'entity.name.type.titrate',
          match: '\\b[A-Z][a-zA-Z0-9_]*\\b',
        },
      ],
    },
  },
}
