# Grammar

## Program

```
program      ::= import* declaration*
import       ::= 'import' IDENTIFIER ('::' IDENTIFIER)* ';'
declaration  ::= access? (fn_decl | class_decl | interface_decl | enum_decl | var_decl | const_decl)
access       ::= 'public' | 'private'
```

## Functions

```
fn_decl      ::= 'fn' IDENTIFIER type_params? '(' params? ')' (':' type)? block
             | type IDENTIFIER '(' sugar_params? ')' block   // sugar form
type_params  ::= '<' type_param (',' type_param)* '>'
type_param   ::= IDENTIFIER (':' type)?
params       ::= param (',' param)*
param        ::= type IDENTIFIER
sugar_params ::= type IDENTIFIER (',' type IDENTIFIER)*
```

## Classes

```
class_decl   ::= 'class' IDENTIFIER type_params? ('extends' type)? ('implements' type (',' type)*)? '{' class_member* '}'
class_member ::= field_decl | method_decl | constructor_decl
field_decl   ::= access? type IDENTIFIER ('=' expr)? ';'
method_decl  ::= access? 'fn' IDENTIFIER '(' params? ')' (':' type)? block
constructor_decl ::= access? IDENTIFIER '(' params? ')' (super_call)? block
super_call   ::= 'super' '(' args? ')' ';'
```

## Interfaces

```
interface_decl ::= 'interface' IDENTIFIER type_params? '{' interface_member* '}'
interface_member ::= access? 'fn' IDENTIFIER '(' params? ')' (':' type)? ';'
```

## Enums

```
enum_decl    ::= 'enum' IDENTIFIER type_params? '{' variant (',' variant)* '}'
variant      ::= IDENTIFIER ('(' type (',' type)* ')')?
```

## Statements

```
stmt         ::= block | expr_stmt | if_stmt | while_stmt | for_stmt
               | return_stmt | break_stmt | continue_stmt | switch_stmt
               | var_decl | const_decl | unsafe_block | region_block
block        ::= '{' stmt* '}'
if_stmt      ::= 'if' '(' expr ')' block ('else' (if_stmt | block))?
while_stmt   ::= 'while' '(' expr ')' block
for_stmt     ::= 'for' '(' ('var')? IDENTIFIER 'in' expr ')' block

> **Note:** Parentheses around the condition/iterator in `if`, `while`, and `for` are optional in the parser but the parenthesized form is the **recommended and preferred style**. Always write `if (expr)`, `while (expr)`, and `for (item in list)`.
return_stmt  ::= 'return' expr? ';'
break_stmt   ::= 'break' ';'
continue_stmt ::= 'continue' ';'
unsafe_block ::= 'unsafe' block
region_block ::= 'region' IDENTIFIER block
```

## Switch

```
switch_stmt  ::= 'switch' expr '{' case_arm* '}'
case_arm     ::= 'case' pattern '=>' stmt
             | 'default' '=>' stmt
pattern      ::= IDENTIFIER '(' (IDENTIFIER)? ')'
             | '_'
```

## Expressions

```
expr         ::= assignment
assignment   ::= ternary (('=' | '+=' | '-=' | '*=' | '/=') assignment)?
ternary      ::= or_expr ('?' expr ':' ternary)?
or_expr      ::= and_expr ('||' and_expr)*
and_expr     ::= equality ('&&' equality)*
equality     ::= comparison (('==' | '!=') comparison)*
comparison   ::= addition (('<' | '>' | '<=' | '>=') addition)*
addition     ::= multiplication (('+' | '-') multiplication)*
multiplication ::= unary (('*' | '/' | '%') unary)*
unary        ::= ('!' | '-' | '&' | '&mut') unary | cast_expr
cast_expr    ::= postfix ('as' type)?
postfix      ::= primary (call | member | index)*
call         ::= '(' args? ')'
member       ::= '.' IDENTIFIER | '::' IDENTIFIER
index        ::= '[' expr ']'
primary      ::= INTEGER | FLOAT | STRING | CHAR | 'true' | 'false' | 'null'
             | 'Ok' '(' expr ')' | 'Err' '(' expr ')'
             | IDENTIFIER ('<' type (',' type)* '>')?
             | '(' expr ')'
             | 'new' type '(' args? ')'
             | 'super' '(' args? ')'
args         ::= expr (',' expr)*
```

## Types

```
type         ::= primitive | 'string' | 'void' | IDENTIFIER ('<' type (',' type)* '>')?
             | 'Owned' '<' type '>' | 'Result' '<' type ',' type '>'
             | 'array' '<' type '>'
primitive    ::= 'bool' | 'byte' | 'short' | 'int' | 'long' | 'vast' | 'uvast'
             | 'float' | 'double' | 'half' | 'quad' | 'char' | 'size'
             | 'u8' | 'u16' | 'u32' | 'u64'
```
