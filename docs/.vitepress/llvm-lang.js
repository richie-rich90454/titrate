export default {
  scopeName: 'source.llvm',
  name: 'llvm',
  patterns: [
    { include: '#comments' },
    { include: '#strings' },
    { include: '#keywords' },
    { include: '#types' },
    { include: '#metadata' },
    { include: '#attributes' },
    { include: '#identifiers' },
    { include: '#numbers' },
    { include: '#operators' },
    { include: '#labels' },
  ],
  repository: {
    comments: {
      patterns: [
        {
          name: 'comment.line.semicolon.llvm',
          match: ';.*$',
        },
      ],
    },
    strings: {
      patterns: [
        {
          name: 'string.quoted.double.llvm',
          begin: '"',
          end: '"',
          patterns: [
            { name: 'constant.character.escape.llvm', match: '\\\\.' },
          ],
        },
        {
          name: 'string.quoted.other.llvm',
          begin: '"',
          end: '"',
        },
      ],
    },
    keywords: {
      patterns: [
        {
          name: 'keyword.control.llvm',
          match: '\\b(define|declare|global|constant|target|module|source_filename|datalayout|triple|attributes|gc|prefix|prologue|personality)\\b',
        },
        {
          name: 'keyword.other.llvm',
          match: '\\b(ret|br|switch|indirectbr|invoke|call|callbr|resume|catchswitch|catchret|cleanupret|unreachable|add|fadd|sub|fsub|mul|fmul|udiv|sdiv|fdiv|urem|srem|frem|shl|lshr|ashr|and|or|xor|icmp|fcmp|phi|select|va_arg|landingpad|catchpad|cleanuppad|alloca|load|store|getelementptr|fence|cmpxchg|atomicrmw|trunc|zext|sext|fptrunc|fpext|fptoui|fptosi|uitofp|sitofp|ptrtoint|inttoptr|bitcast|addrspacecast|extractelement|insertelement|shufflevector|extractvalue|insertvalue|freeze|null|undef|poison|to|nuw|nsw|exact|inbounds|volatile|atomic|syncscope)\\b',
        },
      ],
    },
    types: {
      patterns: [
        {
          name: 'storage.type.llvm',
          match: '\\b(void|half|float|double|fp128|x86_fp80|ppc_fp128|label|metadata|token|ptr|i\\d+|x\\s*\\d+)\\b',
        },
      ],
    },
    metadata: {
      patterns: [
        {
          name: 'constant.numeric.llvm metadata',
          match: '!\\w+',
        },
        {
          name: 'constant.numeric.llvm metadata',
          match: '!\\d+',
        },
      ],
    },
    attributes: {
      patterns: [
        {
          name: 'support.function.llvm',
          match: '\\b(noreturn|nounwind|readnone|readonly|writeonly|argmemonly|inaccessiblememonly|inaccessiblemem_or_argmemonly|returns_twice|zeroext|signext|noalias|nocapture|nonnull|noundef|byval|byref|preallocated|inalloca|sret|align\\d+|align|dereferenceable|dereferenceable_or_null|swiftself|swifterror|immarg|noundef)\\b',
        },
      ],
    },
    identifiers: {
      patterns: [
        {
          name: 'variable.other.llvm',
          match: '\\$\\w+|@%?\\w+|#\\w+',
        },
        {
          name: 'entity.name.function.llvm',
          match: '@\\w+',
        },
      ],
    },
    numbers: {
      patterns: [
        {
          name: 'constant.numeric.llvm',
          match: '\\b-?(\\d+\\.?\\d*|\\.\\d+)([eE][+-]?\\d+)?\\b',
        },
        {
          name: 'constant.numeric.llvm',
          match: '\\b0[xX][0-9a-fA-F]+\\b',
        },
      ],
    },
    operators: {
      patterns: [
        {
          name: 'keyword.operator.llvm',
          match: '(=|\\*|\\+|-|/|%|<<|>>|&|\\||\\^|~|==|!=|<=|>=|<|>)',
        },
      ],
    },
    labels: {
      patterns: [
        {
          name: 'entity.name.label.llvm',
          match: '^\\s*[%-]?\\w+:(?!=)',
        },
      ],
    },
  },
};
