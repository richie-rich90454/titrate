# Grammar

## Program

```
program      ::= import* declaration*
import       ::= 'import' IDENTIFIER ('::' IDENTIFIER)* ';'
declaration  ::= fn_decl | class_decl | interface_decl | enum_decl | var_decl | const_decl
```

## Functions

```
fn_decl      ::= access? 'fn' IDENTIFIER '(' params? ')' (':' type)? block
             | access? type IDENTIFIER '(' sugar_params? ')' block   // sugar form
sugar_params ::= type IDENTIFIER (',' type IDENTIFIER)*
```

## Statements

```
stmt         ::= block | expr_stmt | if_stmt | while_stmt | for_stmt
               | return_stmt | break_stmt | continue_stmt | switch_stmt
               | var_decl | const_decl
block        ::= '{' stmt* '}'
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
unary        ::= ('!' | '-' | '&' | '&mut') unary | postfix
postfix      ::= primary (call | member | index)*
```
