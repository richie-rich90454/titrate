"""
Titrate Alpha 0.1 Interpreter – complete reference implementation.
Usage: python titrate.py <source.tr>
"""

import sys, re, math

# ----------------------------------------------------------------------
#  Lexer
# ----------------------------------------------------------------------
class Token:
    def __init__(self, type_, lexeme=None, value=None, suffix=None):
        self.type = type_
        self.lexeme = lexeme
        self.value = value
        self.suffix = suffix

KEYWORDS = {
    'public':'K_PUB','pub':'K_PUB','private':'K_PRIV','priv':'K_PRIV','protected':'K_PRIV',
    'module':'K_MODULE','import':'K_IMPORT','let':'K_LET','var':'K_VAR','const':'K_CONST',
    'fn':'K_FN','function':'K_FUNCTION','class':'K_CLASS','interface':'K_INTERFACE',
    'enum':'K_ENUM','extends':'K_EXTENDS','implements':'K_IMPLEMENTS','new':'K_NEW',
    'this':'K_THIS','super':'K_SUPER','void':'K_VOID','bool':'K_BOOL','byte':'K_BYTE',
    'short':'K_SHORT','int':'K_INT','long':'K_LONG','vast':'K_VAST','uvast':'K_UVAST',
    'float':'K_FLOAT','double':'K_DOUBLE','half':'K_HALF','quad':'K_QUAD','char':'K_CHAR',
    'string':'K_STRING','size':'K_SIZE','Owned':'K_OWNED','region':'K_REGION',
    'unsafe':'K_UNSAFE','Result':'K_RESULT','Ok':'K_OK','Err':'K_ERR','if':'K_IF',
    'else':'K_ELSE','while':'K_WHILE','for':'K_FOR','return':'K_RETURN','break':'K_BREAK',
    'continue':'K_CONTINUE','switch':'K_SWITCH','case':'K_CASE','default':'K_DEFAULT',
    'true':'K_TRUE','false':'K_FALSE','null':'K_NULL','as':'K_AS','is':'K_IS','type':'K_TYPE',
}

PUNCT = { '+':'PLUS', '-':'MINUS', '*':'STAR', '/':'SLASH', '%':'PERCENT',
          '=':'ASSIGN', '==':'EQEQ', '!=':'NEQ', '<':'LT', '>':'GT',
          '<=':'LEQ', '>=':'GEQ', '&&':'ANDAND', '||':'OROR', '!':'BANG',
          '&':'AMPERSAND', '?':'QUESTION', '.':'DOT', ',':'COMMA', ';':'SEMI',
          ':':'COLON', '::':'COLONCOLON', '->':'ARROW', '=>':'FATARROW',
          '(':'LPAREN', ')':'RPAREN', '{':'LBRACE', '}':'RBRACE',
          '[':'LBRACKET', ']':'RBRACKET', '&mut':'AMPERSAND_MUT' }

class Lexer:
    def __init__(self, src):
        self.src = src
        self.pos = 0

    def tokenize(self):
        tokens = []
        while self.pos < len(self.src):
            self.skip_ws()
            if self.pos >= len(self.src): break
            ch = self.src[self.pos]
            if ch == '"': tokens.append(self.read_string())
            elif ch == "'": tokens.append(self.read_char())
            elif ch.isdigit() or (ch == '.' and self.pos+1 < len(self.src) and self.src[self.pos+1].isdigit()):
                tokens.append(self.read_number())
            elif ch.isalpha() or ch == '_': tokens.append(self.read_ident_or_kw())
            else: tokens.append(self.read_punct())
        return tokens

    def skip_ws(self):
        while self.pos < len(self.src):
            if self.src[self.pos] in ' \t\n\r':
                self.pos += 1
                continue
            if self.src[self.pos] == '/' and self.pos+1 < len(self.src):
                if self.src[self.pos+1] == '/':
                    while self.pos < len(self.src) and self.src[self.pos] != '\n':
                        self.pos += 1
                    continue
                if self.src[self.pos+1] == '*':
                    self.pos += 2
                    while self.pos < len(self.src)-1 and not (self.src[self.pos]=='*' and self.src[self.pos+1]=='/'):
                        self.pos += 1
                    self.pos += 2
                    continue
            break

    def read_string(self):
        self.pos += 1
        s = ''
        while self.pos < len(self.src) and self.src[self.pos] != '"':
            if self.src[self.pos] == '\\':
                self.pos += 1
                esc = self.src[self.pos]
                s += {'n':'\n','t':'\t','\\':'\\','"':'"'}.get(esc, esc)
            else:
                s += self.src[self.pos]
            self.pos += 1
        self.pos += 1
        return Token('STRING', lexeme=f'"{s}"', value=s)

    def read_char(self):
        self.pos += 1
        if self.src[self.pos] == '\\':
            self.pos += 1
            ch = {'n':'\n','t':'\t','\\':'\\'}.get(self.src[self.pos], self.src[self.pos])
        else:
            ch = self.src[self.pos]
        self.pos += 2  # char + closing '
        return Token('CHAR', value=ch)

    def read_number(self):
        raw = ''
        if self.src[self.pos] == '0' and self.pos+1 < len(self.src) and self.src[self.pos+1] in 'xX':
            raw += self.src[self.pos]; self.pos += 1
            raw += self.src[self.pos]; self.pos += 1
            while self.pos < len(self.src) and self.src[self.pos] in '0123456789abcdefABCDEF':
                raw += self.src[self.pos]; self.pos += 1
            val = int(raw, 16)
            return Token('NUMBER', lexeme=raw, value=val)

        if self.src[self.pos] == '.':
            raw += '.'; self.pos += 1
        while self.pos < len(self.src) and self.src[self.pos].isdigit():
            raw += self.src[self.pos]; self.pos += 1
        if self.pos < len(self.src) and self.src[self.pos] == '.' and '.' not in raw:
            raw += '.'; self.pos += 1
            while self.pos < len(self.src) and self.src[self.pos].isdigit():
                raw += self.src[self.pos]; self.pos += 1
        if self.pos < len(self.src) and self.src[self.pos] in 'eE':
            raw += self.src[self.pos]; self.pos += 1
            if self.src[self.pos] in '+-':
                raw += self.src[self.pos]; self.pos += 1
            while self.pos < len(self.src) and self.src[self.pos].isdigit():
                raw += self.src[self.pos]; self.pos += 1
        suffix = None
        if self.pos < len(self.src) and self.src[self.pos] in 'hq':
            suffix = self.src[self.pos]; raw += suffix; self.pos += 1
        num_part = raw[:-1] if suffix else raw
        try: val = int(num_part)
        except: val = float(num_part)
        return Token('NUMBER', lexeme=raw, value=val, suffix=suffix)

    def read_ident_or_kw(self):
        start = self.pos
        while self.pos < len(self.src) and (self.src[self.pos].isalnum() or self.src[self.pos] == '_'):
            self.pos += 1
        word = self.src[start:self.pos]
        if word in KEYWORDS:
            return Token(KEYWORDS[word], lexeme=word)
        return Token('IDENTIFIER', lexeme=word)

    def read_punct(self):
        for size in (4, 2, 1):
            if size == 4 and self.src[self.pos:self.pos+4] == '&mut':
                self.pos += 4; return Token('AMPERSAND_MUT')
            if size == 2:
                p = self.src[self.pos:self.pos+2]
                if p in PUNCT:
                    self.pos += 2; return Token(PUNCT[p])
            if size == 1:
                p = self.src[self.pos]
                if p in PUNCT:
                    self.pos += 1; return Token(PUNCT[p])
        raise ValueError(f'Unexpected character {self.src[self.pos]}')

# ----------------------------------------------------------------------
#  Parser
# ----------------------------------------------------------------------
class Parser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.pos = 0
        self.current_class = None

    def peek(self): return self.tokens[self.pos] if self.pos < len(self.tokens) else None
    def check(self, t): return self.peek() and self.peek().type == t
    def advance(self):
        t = self.peek()
        if t: self.pos += 1
        return t
    def match(self, *types):
        for t in types:
            if self.check(t):
                self.advance()
                return True
        return False
    def consume(self, t, msg=''):
        if self.check(t): return self.advance()
        raise SyntaxError(msg or f'Expected {t}, got {self.peek().type}')

    def program(self):
        stmts = []
        while self.peek() is not None:
            stmts.append(self.declaration())
        return ('Program', stmts)

    def declaration(self):
        vis = self.parse_visibility()
        if self.match('K_MODULE', 'K_IMPORT'):
            while not self.check('SEMI') and self.peek() is not None:
                self.advance()
            if self.check('SEMI'):
                self.advance()
            return ('Empty',)
        if self.match('K_CLASS'): return self.class_decl(vis)
        if self.match('K_INTERFACE'): return self.iface_decl(vis)
        if self.match('K_ENUM'): return self.enum_decl(vis)
        if self.match('K_FN','K_FUNCTION'): return self.fn_decl(vis)
        if self.is_type_start() or self.check('IDENTIFIER'): return self.sugar_decl(vis)
        return self.statement()

    def parse_visibility(self):
        if self.match('K_PUB'): return 'public'
        if self.match('K_PRIV'): return 'private'
        return None

    def is_type_start(self):
        return self.peek() and self.peek().type in (
            'K_VOID','K_BOOL','K_BYTE','K_SHORT','K_INT','K_LONG','K_VAST','K_UVAST',
            'K_FLOAT','K_DOUBLE','K_HALF','K_QUAD','K_CHAR','K_STRING','K_SIZE','K_OWNED',
            'STAR','AMPERSAND_MUT','AMPERSAND'
        )

    def sugar_decl(self, vis):
        pos = self.pos
        typ = self.parse_type()
        if not self.check('IDENTIFIER'):
            self.pos = pos; return self.statement()
        name = self.advance().lexeme
        if self.match('LPAREN'):
            params = self.parse_params()
            body = self.block() if self.check('LBRACE') else (self.consume('SEMI'), None)[1]
            return ('FunctionDecl', name, params, typ, body, vis, True)
        elif self.match('ASSIGN'):
            init = self.expression()
            self.consume('SEMI')
            return ('VarDecl', True, name, typ, init, vis)
        else:
            self.consume('SEMI')
            return ('VarDecl', True, name, typ, None, vis)

    def parse_params(self):
        params = []
        if self.check('RPAREN'): self.advance(); return params
        while True:
            params.append(self.parse_param())
            if not self.match('COMMA'): break
        self.consume('RPAREN')
        return params

    def parse_param(self):
        if self.check('IDENTIFIER'):
            nxt = self.tokens[self.pos+1] if self.pos+1 < len(self.tokens) else None
            if nxt and nxt.type in ('COLON','COMMA','RPAREN'):
                name = self.advance().lexeme
                if self.match('COLON'): return (name, self.parse_type())
                return (name, None)
        typ = self.parse_type()
        name = self.consume('IDENTIFIER').lexeme
        return (name, typ)

    def parse_type(self):
        if self.match('STAR','AMPERSAND','AMPERSAND_MUT'): return self.parse_type()
        if not self.peek(): raise SyntaxError('Expected type')
        if not self.is_type_start() and not self.check('IDENTIFIER'):
            raise SyntaxError(f'Expected type, got {self.peek().type}')
        base = self.advance().lexeme
        args = []
        if self.match('LT'):
            while True:
                args.append(self.parse_type())
                if not self.match('COMMA'): break
            self.consume('GT')
        return (base, args)

    def fn_decl(self, vis):
        name = self.parse_fn_name()
        self.consume('LPAREN')
        params = self.parse_params()
        ret_type = None
        if self.match('COLON'):
            ret_type = self.parse_type()
        body = self.block()
        return ('FunctionDecl', name, params, ret_type, body, vis, False)

    def parse_fn_name(self):
        if self.check('IDENTIFIER') or self.check('K_NEW'):
            return self.advance().lexeme
        raise SyntaxError('Expected function name')

    def class_decl(self, vis):
        name = self.consume('IDENTIFIER').lexeme
        parent = None
        if self.match('K_EXTENDS'): parent = self.parse_type()
        ifaces = []
        if self.match('K_IMPLEMENTS'):
            while True:
                ifaces.append(self.parse_type())
                if not self.match('COMMA'): break
        self.consume('LBRACE')
        prev = self.current_class
        self.current_class = name
        members = []
        while not self.check('RBRACE') and self.peek():
            members.append(self.class_member())
        self.consume('RBRACE')
        self.current_class = prev
        return ('ClassDecl', name, parent, ifaces, members, vis)

    def class_member(self):
        vis = self.parse_visibility()
        if self.check('IDENTIFIER') and self.peek().lexeme == self.current_class and \
           self.pos+1 < len(self.tokens) and self.tokens[self.pos+1].type == 'LPAREN':
            name = self.advance().lexeme
            self.consume('LPAREN')
            params = self.parse_params()
            body = self.block()
            return ('Constructor', params, body, vis)
        if self.match('K_FN','K_FUNCTION'): return self.fn_decl(vis)

        if self.check('IDENTIFIER'):
            nxt = self.tokens[self.pos+1] if self.pos+1 < len(self.tokens) else None
            if nxt and nxt.type == 'COLON':
                name = self.advance().lexeme
                self.consume('COLON')
                typ = self.parse_type()
                init = None
                if self.match('ASSIGN'): init = self.expression()
                self.consume('SEMI')
                return ('VarDecl', True, name, typ, init, vis)

        typ = self.parse_type()
        name = self.consume('IDENTIFIER').lexeme
        if self.match('LPAREN'):
            params = self.parse_params()
            body = self.block() if self.check('LBRACE') else (self.consume('SEMI'), None)[1]
            return ('FunctionDecl', name, params, typ, body, vis, True)
        else:
            init = None
            if self.match('ASSIGN'): init = self.expression()
            self.consume('SEMI')
            return ('VarDecl', True, name, typ, init, vis)

    def iface_decl(self, vis):
        name = self.consume('IDENTIFIER').lexeme
        parents = []
        if self.match('K_EXTENDS'):
            while True:
                parents.append(self.parse_type())
                if not self.match('COMMA'): break
        self.consume('LBRACE')
        methods = []
        while not self.check('RBRACE') and self.peek():
            v = self.parse_visibility()
            self.consume('K_FN')
            n = self.consume('IDENTIFIER').lexeme
            self.consume('LPAREN')
            pr = self.parse_params()
            self.consume('COLON')
            rt = self.parse_type()
            self.consume('SEMI')
            methods.append(('FunctionDecl', n, pr, rt, None, v, False))
        self.consume('RBRACE')
        return ('InterfaceDecl', name, parents, methods, vis)

    def enum_decl(self, vis):
        name = self.consume('IDENTIFIER').lexeme
        self.consume('LBRACE')
        variants = []
        while not self.check('RBRACE') and self.peek():
            vn = self.consume('IDENTIFIER').lexeme
            fields = []
            if self.match('LPAREN'):
                if not self.check('RPAREN'):
                    while True:
                        fn = self.consume('IDENTIFIER').lexeme
                        self.consume('COLON')
                        ft = self.parse_type()
                        fields.append((fn, ft))
                        if not self.match('COMMA'): break
                self.consume('RPAREN')
            self.consume('SEMI')
            variants.append((vn, fields))
        self.consume('RBRACE')
        return ('EnumDecl', name, variants, vis)

    # ----------------- statements -----------------
    def statement(self):
        if self.match('K_IF'): return self.if_stmt()
        if self.match('K_WHILE'): return self.while_stmt()
        if self.match('K_FOR'): return self.for_stmt()
        if self.match('K_RETURN'):
            val = None
            if not self.check('SEMI'): val = self.expression()
            self.consume('SEMI')
            return ('Return', val)
        if self.match('K_BREAK'): self.consume('SEMI'); return ('Break',)
        if self.match('K_CONTINUE'): self.consume('SEMI'); return ('Continue',)
        if self.match('K_SWITCH'): return self.switch_stmt()
        if self.match('K_REGION'): return ('Region', self.block())
        if self.match('K_UNSAFE'): return ('Unsafe', self.block())
        if self.match('LBRACE'): return self.block()
        if self.match('K_LET','K_VAR','K_CONST'):
            mut = self.tokens[self.pos-1].type != 'K_CONST'
            name = self.consume('IDENTIFIER').lexeme
            vt = None
            if self.match('COLON'): vt = self.parse_type()
            init = None
            if self.match('ASSIGN'): init = self.expression()
            self.consume('SEMI')
            return ('VarDecl', mut, name, vt, init, None)
        expr = self.expression()
        self.consume('SEMI')
        return ('ExprStmt', expr)

    def block(self):
        self.consume('LBRACE')
        stmts = []
        while not self.check('RBRACE') and self.peek():
            stmts.append(self.declaration())
        self.consume('RBRACE')
        return ('Block', stmts)

    def if_stmt(self):
        self.consume('LPAREN'); cond = self.expression(); self.consume('RPAREN')
        then = self.statement()
        els = self.statement() if self.match('K_ELSE') else None
        return ('If', cond, then, els)

    def while_stmt(self):
        self.consume('LPAREN'); cond = self.expression(); self.consume('RPAREN')
        body = self.statement()
        return ('While', cond, body)

    def for_stmt(self):
        self.consume('LPAREN')
        init = None
        if self.match('K_LET','K_VAR','K_CONST'):
            mut = self.tokens[self.pos-1].type != 'K_CONST'
            name = self.consume('IDENTIFIER').lexeme
            vt = None
            if self.match('COLON'):
                vt = self.parse_type()
            ie = None
            if self.match('ASSIGN'):
                ie = self.expression()
            init = ('VarDecl', mut, name, vt, ie, None)
        elif not self.check('SEMI'):
            init = self.expression()
        self.consume('SEMI')
        cond = None if self.check('SEMI') else self.expression()
        self.consume('SEMI')
        step = None if self.check('RPAREN') else self.expression()
        self.consume('RPAREN')
        body = self.statement()
        return ('For', init, cond, step, body)

    def switch_stmt(self):
        self.consume('LPAREN'); expr = self.expression(); self.consume('RPAREN')
        self.consume('LBRACE')
        cases = []
        default = None
        while not self.check('RBRACE') and self.peek():
            if self.match('K_DEFAULT'):
                self.consume('FATARROW'); default = self.statement(); break
            self.consume('K_CASE')
            pat = self.parse_pattern()
            self.consume('FATARROW')
            body = self.statement()
            cases.append((pat, body))
        self.consume('RBRACE')
        return ('Switch', expr, cases, default)

    def parse_pattern(self):
        if self.check('NUMBER'): return ('LiteralPattern', self.advance().value)
        if self.check('STRING'): return ('LiteralPattern', self.advance().value)
        if self.check('CHAR'):   return ('LiteralPattern', self.advance().value)
        if self.match('K_TRUE'): return ('LiteralPattern', True)
        if self.match('K_FALSE'): return ('LiteralPattern', False)
        if self.match('K_NULL'): return ('LiteralPattern', None)
        if self.match('K_OK'):
            self.consume('LPAREN'); sub = self.parse_pattern(); self.consume('RPAREN')
            return ('ConstructorPattern', 'Ok', [sub])
        if self.match('K_ERR'):
            self.consume('LPAREN'); sub = self.parse_pattern(); self.consume('RPAREN')
            return ('ConstructorPattern', 'Err', [sub])
        path = self.consume('IDENTIFIER').lexeme
        while self.match('DOT','COLONCOLON'):
            path += '.' + self.consume('IDENTIFIER').lexeme
        if self.match('LPAREN'):
            args = []
            if not self.check('RPAREN'):
                while True:
                    self.consume('IDENTIFIER'); args.append(self.tokens[self.pos-1].lexeme)
                    if not self.match('COMMA'): break
            self.consume('RPAREN')
            return ('ConstructorPattern', path, args)
        return ('VariablePattern', path)

    # ----------------- expressions -----------------
    def expression(self): return self.assignment()

    def assignment(self):
        left = self.ternary()
        if self.match('ASSIGN'):
            right = self.assignment()
            return ('Assign', left, right)
        return left

    def ternary(self):
        left = self.logic_or()
        if self.match('QUESTION') and self.peek() and self.peek().type != 'QUESTION':
            then = self.expression()
            self.consume('COLON')
            els = self.ternary()
            return ('Ternary', left, then, els)
        return left

    def logic_or(self):
        left = self.logic_and()
        while self.match('OROR'):
            right = self.logic_and()
            left = ('Binary', '||', left, right)
        return left

    def logic_and(self):
        left = self.equality()
        while self.match('ANDAND'):
            right = self.equality()
            left = ('Binary', '&&', left, right)
        return left

    def equality(self):
        left = self.comparison()
        while self.match('EQEQ','NEQ'):
            op = '==' if self.tokens[self.pos-1].type == 'EQEQ' else '!='
            right = self.comparison()
            left = ('Binary', op, left, right)
        return left

    def comparison(self):
        left = self.addition()
        while self.match('LT','GT','LEQ','GEQ'):
            m = {'LT':'<','GT':'>','LEQ':'<=','GEQ':'>='}
            right = self.addition()
            left = ('Binary', m[self.tokens[self.pos-1].type], left, right)
        return left

    def addition(self):
        left = self.multiplication()
        while self.match('PLUS','MINUS'):
            op = '+' if self.tokens[self.pos-1].type == 'PLUS' else '-'
            right = self.multiplication()
            left = ('Binary', op, left, right)
        return left

    def multiplication(self):
        left = self.unary()
        while self.match('STAR','SLASH','PERCENT'):
            m = {'STAR':'*','SLASH':'/','PERCENT':'%'}
            right = self.unary()
            left = ('Binary', m[self.tokens[self.pos-1].type], left, right)
        return left

    def unary(self):
        if self.match('MINUS','BANG','AMPERSAND_MUT','AMPERSAND','STAR'):
            op = self.tokens[self.pos-1].type
            e = self.unary()
            if op == 'STAR': return ('Deref', e)
            if op == 'AMPERSAND_MUT': return ('RefMut', e)
            if op == 'AMPERSAND': return ('Ref', e)
            if op == 'MINUS': return ('Unary', '-', e)
            if op == 'BANG': return ('Unary', '!', e)
        return self.postfix()

    def postfix(self):
        left = self.call_member()
        while True:
            if self.match('QUESTION'): left = ('ErrProp', left)
            elif self.match('K_AS'): self.parse_type()
            else: break
        return left

    def call_member(self):
        left = self.primary()
        while True:
            if self.match('DOT','COLONCOLON'):
                mem = self.consume('IDENTIFIER').lexeme
                ta = None
                if self.match('LT'):
                    ta = []
                    while True:
                        ta.append(self.parse_type())
                        if not self.match('COMMA'): break
                    self.consume('GT')
                if self.match('LPAREN'):
                    args = self.parse_args()
                    left = ('MethodCall', left, mem, ta, args)
                elif ta:
                    left = ('GenericAccess', left, mem, ta)
                else:
                    left = ('Member', left, mem)
            elif self.match('LPAREN'):
                args = self.parse_args()
                left = ('Call', left, args)
            elif self.match('LBRACKET'):
                idx = self.expression()
                self.consume('RBRACKET')
                left = ('Index', left, idx)
            else:
                break
        return left

    def parse_args(self):
        args = []
        if self.check('RPAREN'): self.advance(); return args
        while True:
            args.append(self.expression())
            if not self.match('COMMA'): break
        self.consume('RPAREN')
        return args

    def primary(self):
        if self.match('K_TRUE'): return ('Literal', True)
        if self.match('K_FALSE'): return ('Literal', False)
        if self.match('K_NULL'): return ('Literal', None)
        if self.check('NUMBER'):
            t = self.advance()
            return ('Literal', t.value, t.suffix)
        if self.match('STRING'): return ('Literal', self.tokens[self.pos-1].value)
        if self.match('CHAR'):   return ('Literal', self.tokens[self.pos-1].value)
        if self.match('K_THIS'): return ('This',)
        if self.match('K_SUPER'):
            if self.match('DOT','COLONCOLON'):
                mem = self.consume('IDENTIFIER').lexeme
                if self.match('LPAREN'):
                    args = self.parse_args()
                    return ('SuperMethodCall', mem, args)
                return ('SuperAccess', mem)
            self.consume('LPAREN')
            args = self.parse_args()
            return ('SuperCall', args)
        if self.match('K_NEW'):
            tp = self.parse_type()
            ta = None
            if self.match('LT'):
                ta = []
                while True:
                    ta.append(self.parse_type())
                    if not self.match('COMMA'): break
                self.consume('GT')
            self.consume('LPAREN')
            args = self.parse_args()
            return ('New', tp, ta, args)
        if self.match('K_OK'):
            self.consume('LPAREN'); v = self.expression(); self.consume('RPAREN')
            return ('OkExpr', v)
        if self.match('K_ERR'):
            self.consume('LPAREN'); v = self.expression(); self.consume('RPAREN')
            return ('ErrExpr', v)
        if self.match('IDENTIFIER'): return ('Identifier', self.tokens[self.pos-1].lexeme)
        if self.match('LPAREN'):
            e = self.expression()
            self.consume('RPAREN')
            return e
        raise SyntaxError(f'Unexpected token {self.peek().type}')

# ----------------------------------------------------------------------
#  Evaluator
# ----------------------------------------------------------------------
class Environment:
    def __init__(self, parent=None):
        self.parent = parent
        self.vars = {}

    def define(self, name, value): self.vars[name] = value
    def assign(self, name, value):
        if name in self.vars: self.vars[name] = value; return
        if self.parent: self.parent.assign(name, value)
        else: raise NameError(f"'{name}' not defined")
    def get(self, name):
        if name in self.vars: return self.vars[name]
        if self.parent: return self.parent.get(name)
        raise NameError(f"'{name}' not defined")

class ReturnSignal(Exception):
    def __init__(self, value): self.value = value
class BreakSignal(Exception): pass
class ContinueSignal(Exception): pass

class Evaluator:
    def __init__(self, ast):
        self.ast = ast
        self.env = Environment()
        self.current_fn = None
        self._setup_builtins()

    def _setup_builtins(self):
        g = self.env
        g.define('io', type('io', (), {'println': lambda self, *a: print(' '.join(map(str, a)))})())
        g.define('Integer', type('Integer', (), {
            'toString': staticmethod(lambda v: str(v)),
            'parseInt': staticmethod(lambda s: ({'_ok': True, 'value': int(s)} if s.lstrip('-').isdigit() else {'_ok': False, 'value': 'parse error'}))
        })())
        g.define('Double', type('Double', (), {'toString': staticmethod(lambda v: str(v))})())
        for cls in ['Character','String','Byte','Short','Half','Quad','Vast','Uvast']:
            g.define(cls, type(cls, (), {'toString': staticmethod(lambda v, cls=cls: str(v))})())
        g.define('region', type('region', (), {'alloc': staticmethod(lambda sz: [None]*sz)}))
        g.define('malloc', lambda sz: type('Ptr', (), {'ptr': bytearray(sz), 'view': None})())
        g.define('free', lambda p: None)
        g.define('size', object())
        class ArrayList(list):
            def add(self, e): self.append(e)
            def get(self, i): return self[i]
            def size(self): return len(self)
            def sort(self): super().sort()
        class HashMap(dict):
            def put(self, k, v): self[k] = v
            def get(self, k): return self.get(k)
        g.define('ArrayList', type('ArrayList', (), {'__class__': True, 'ctor': ArrayList}))
        g.define('HashMap', type('HashMap', (), {'__class__': True, 'ctor': HashMap}))
        g.define('array', type('array', (), {'__builtin__': True})())

    def run(self):
        self.exec_block(self.ast[1], self.env)
        if 'main' in self.env.vars:
            self.call(self.env.get('main'), [])

    def exec_block(self, stmts, env):
        prev = self.env
        self.env = env
        try:
            for stmt in stmts:
                self.execute(stmt)
        finally:
            self.env = prev

    def execute(self, node):
        tag = node[0]
        if tag == 'ExprStmt': self.eval(node[1])
        elif tag == 'VarDecl': self.var_decl(node)
        elif tag == 'FunctionDecl': self.func_decl(node)
        elif tag == 'ClassDecl': self.class_decl(node)
        elif tag == 'EnumDecl': self.enum_decl(node)
        elif tag == 'If': self.if_stmt(node)
        elif tag == 'While': self.while_stmt(node)
        elif tag == 'For': self.for_stmt(node)
        elif tag == 'Return': raise ReturnSignal(self.eval(node[1]) if node[1] is not None else None)
        elif tag == 'Break': raise BreakSignal()
        elif tag == 'Continue': raise ContinueSignal()
        elif tag == 'Switch': self.switch_stmt(node)
        elif tag == 'Block': self.exec_block(node[1], Environment(self.env))
        elif tag == 'Region': self.exec_block(node[1][1], Environment(self.env))
        elif tag == 'Unsafe': self.exec_block(node[1][1], Environment(self.env))
        elif tag == 'Empty': pass
        else: raise NotImplementedError(tag)

    def var_decl(self, node):
        _, mut, name, typ, init, vis = node
        val = self.eval(init) if init is not None else None
        self.env.define(name, val)

    def func_decl(self, node):
        _, name, params, ret, body, vis, sugar = node
        f = ('func', params, body, self.env)
        self.env.define(name, f)

    def class_decl(self, node):
        _, name, parent, ifaces, members, vis = node
        cd = {'type': 'class', 'name': name, 'parent': None, 'methods': {}, 'ctor': None}
        for m in members:
            if m[0] == 'FunctionDecl':
                fn = self._make_fun(m, cd)
                cd['methods'][m[1]] = fn
            elif m[0] == 'Constructor':
                fn = self._make_fun(('FunctionDecl', 'constructor', m[1], None, m[2], m[3], False), cd)
                cd['ctor'] = fn
        if parent:
            pcls = self.env.get(parent[0])
            cd['parent'] = pcls
        self.env.define(name, cd)

    def _make_fun(self, node, cls):
        _, name, params, ret, body, vis, sugar = node
        return ('func', params, body, self.env, cls)

    def enum_decl(self, node):
        _, name, variants, vis = node
        eo = {'type': 'enum', 'name': name, 'variants': {}}
        for vn, fields in variants:
            def ctor(*args, vn=vn, fields=fields, eo=eo):
                obj = {'_enum': eo, '_variant': vn}
                for i, (fn, _) in enumerate(fields):
                    obj[fn] = args[i]
                return obj
            eo['variants'][vn] = ctor
            eo[vn] = ctor
        self.env.define(name, eo)

    def if_stmt(self, node):
        if self.truthy(self.eval(node[1])):
            self.execute(node[2])
        elif node[3]:
            self.execute(node[3])

    def while_stmt(self, node):
        while self.truthy(self.eval(node[1])):
            try: self.execute(node[2])
            except BreakSignal: break
            except ContinueSignal: continue

    def for_stmt(self, node):
        _, init, cond, step, body = node
        env = Environment(self.env)
        self.env = env
        try:
            if init: self.execute(init)
            while True:
                if cond and not self.truthy(self.eval(cond)): break
                try: self.execute(body)
                except BreakSignal: break
                except ContinueSignal: pass
                if step: self.eval(step)
        finally:
            self.env = env.parent

    def switch_stmt(self, node):
        _, expr, cases, default = node
        val = self.eval(expr)
        for pat, body in cases:
            bind = self.match_pattern(pat, val)
            if bind is not None:
                env = Environment(self.env)
                self.env = env
                try:
                    for k, v in bind.items():
                        env.define(k, v)
                    self.execute(body)
                    return
                finally:
                    self.env = env.parent
        if default:
            self.execute(default)

    def match_pattern(self, pat, val):
        tag = pat[0]
        if tag == 'LiteralPattern':
            return {} if val == pat[1] else None
        if tag == 'ConstructorPattern':
            if not isinstance(val, dict) or '_variant' not in val: return None
            pname = pat[1]
            parts = pname.split('.')
            vname = parts[1] if len(parts) > 1 else parts[0]
            if val['_variant'] != vname: return None
            bind = {}
            for i, arg in enumerate(pat[2]):
                bind[arg] = val.get( list(val.keys())[i] )
            return bind
        if tag == 'VariablePattern':
            return {pat[1]: val}
        return None

    def eval(self, node):
        tag = node[0]
        if tag == 'Literal': return node[1]
        if tag == 'Identifier': return self.env.get(node[1])
        if tag == 'This': return self.env.get('this')
        if tag == 'Unary':
            v = self.eval(node[2])
            return -v if node[1] == '-' else not v
        if tag == 'Binary':
            l = self.eval(node[2]); r = self.eval(node[3])
            op = node[1]
            if op == '+': return self.add(l, r)
            if op == '-': return l - r
            if op == '*': return l * r
            if op == '/': return l / r
            if op == '%': return l % r
            if op == '==': return self.eq(l, r)
            if op == '!=': return not self.eq(l, r)
            if op == '<': return l < r
            if op == '>': return l > r
            if op == '<=': return l <= r
            if op == '>=': return l >= r
            if op == '&&': return l and r
            if op == '||': return l or r
        if tag == 'Ternary':
            return self.eval(node[2]) if self.truthy(self.eval(node[1])) else self.eval(node[3])
        if tag == 'Assign':
            val = self.eval(node[2])
            target = node[1]
            if target[0] == 'Identifier': self.env.assign(target[1], val)
            elif target[0] == 'Member': self.eval(target[1]).__dict__[target[2]] = val
            elif target[0] == 'Index': self.eval(target[1])[self.eval(target[2])] = val
            elif target[0] == 'Deref': pass
            return val
        if tag == 'Member':
            obj = self.eval(node[1])
            if hasattr(obj, node[2]): return getattr(obj, node[2])
            return obj[node[2]]
        if tag == 'Index':
            return self.eval(node[1])[self.eval(node[2])]
        if tag == 'Call':
            fn = self.eval(node[1]); args = [self.eval(a) for a in node[2]]
            return self.call(fn, args)
        if tag == 'MethodCall':
            obj = self.eval(node[1]); args = [self.eval(a) for a in node[3]]
            meth = getattr(obj, node[2])
            return meth(*args)
        if tag == 'New':
            cls = node[1][0]
            if cls == 'ArrayList': return self.env.get('ArrayList').ctor()
            if cls == 'HashMap': return self.env.get('HashMap').ctor()
            if cls == 'array':
                sz = self.eval(node[3][0])
                return [None] * sz
            cd = self.env.get(cls)
            obj = object.__new__(type(cls, (), {}))
            ctor = cd.get('ctor')
            if ctor:
                self.call(ctor, [self.eval(a) for a in node[3]], this=obj)
            return obj
        if tag == 'Deref': return self.eval(node[1])
        if tag == 'RefMut': return self.eval(node[1])
        if tag == 'ErrProp':
            r = self.eval(node[1])
            if isinstance(r, dict) and r.get('_ok') is False: raise ReturnSignal(r)
            return r['value'] if isinstance(r, dict) else r
        if tag == 'OkExpr': return {'_ok': True, 'value': self.eval(node[1])}
        if tag == 'ErrExpr': return {'_ok': False, 'value': self.eval(node[1])}
        if tag == 'SuperCall':
            cls = self.current_fn[3] if self.current_fn else None
            if cls and cls.get('parent'):
                ctor = cls['parent'].get('ctor')
                if ctor:
                    return self.call(ctor, [self.eval(a) for a in node[1]], this=self.env.get('this'))
        if tag == 'SuperMethodCall':
            cls = self.current_fn[3] if self.current_fn else None
            if cls and cls.get('parent'):
                meth = cls['parent']['methods'][node[1]]
                if meth:
                    return self.call(meth, [self.eval(a) for a in node[2]], this=self.env.get('this'))
        if tag == 'SuperAccess':
            cls = self.current_fn[3] if self.current_fn else None
            if cls and cls.get('parent'):
                return cls['parent']['methods'][node[1]]
        raise NotImplementedError(tag)

    def call(self, fn, args, this=None):
        if callable(fn):
            return fn(*args)
        if fn[0] != 'func': raise TypeError('Not a function')
        _, params, body, closure = fn[:4]
        env = Environment(closure)
        if this: env.define('this', this)
        for (pname, ptype), arg in zip(params, args):
            env.define(pname, arg)
        prev_env = self.env
        prev_fn = self.current_fn
        self.env = env
        self.current_fn = fn
        try:
            self.exec_block(body[1], env)
            return None
        except ReturnSignal as ret:
            return ret.value
        finally:
            self.env = prev_env
            self.current_fn = prev_fn

    def truthy(self, v): return bool(v)
    def add(self, a, b):
        if isinstance(a, (int, float)) and isinstance(b, (int, float)): return a + b
        return str(a) + str(b)
    def eq(self, a, b):
        return a == b

# ----------------------------------------------------------------------
#  Main
# ----------------------------------------------------------------------
if __name__ == '__main__':
    if len(sys.argv) < 2:
        print('Usage: python titrate.py <source.tr>')
        sys.exit(1)
    src = open(sys.argv[1], encoding='utf-8').read()
    tokens = Lexer(src).tokenize()
    ast = Parser(tokens).program()
    Evaluator(ast).run()