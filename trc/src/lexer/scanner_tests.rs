use super::scanner::tokenize;
use super::token::{FloatSuffix, Token};
use super::SpannedToken;


    #[allow(dead_code)]
    fn tok(token: Token) -> SpannedToken {
        SpannedToken {
            token,
            line: 0,
            column: 0,
        }
    }

    #[test]
    fn test_micro_test() {
        let src = r#"public fn main(): void { io::println("Hello"); }"#;
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();

        let main_id = Token::Identifier("main".to_string());
        let io_id = Token::Identifier("io".to_string());
        let println_id = Token::Identifier("println".to_string());
        let hello_str = Token::StringLiteral("Hello".to_string());

        let expected = vec![
            &Token::Public,
            &Token::Fn,
            &main_id,
            &Token::LeftParen,
            &Token::RightParen,
            &Token::Colon,
            &Token::Void,
            &Token::LeftBrace,
            &io_id,
            &Token::ColonColon,
            &println_id,
            &Token::LeftParen,
            &hello_str,
            &Token::RightParen,
            &Token::Semicolon,
            &Token::RightBrace,
            &Token::Eof,
        ];

        assert_eq!(token_kinds, expected);
    }

    #[test]
    fn test_integer_literals() {
        let src = "42 0xFF 0o77 0b1010";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let values: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(values[0], &Token::IntLiteral(42));
        assert_eq!(values[1], &Token::IntLiteral(255));
        assert_eq!(values[2], &Token::IntLiteral(63));
        assert_eq!(values[3], &Token::IntLiteral(10));
    }

    #[test]
    fn test_float_with_suffix() {
        let src = "1.5h 2.0q 3.14";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let values: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(values[0], &Token::FloatLiteral { value: 1.5, suffix: Some(FloatSuffix::Half) });
        assert_eq!(values[1], &Token::FloatLiteral { value: 2.0, suffix: Some(FloatSuffix::Quad) });
        // 3.14 is the tokenized fixture input, not an approximation of PI.
        #[allow(clippy::approx_constant)]
        {
            assert_eq!(values[2], &Token::FloatLiteral { value: 3.14, suffix: None });
        }
    }

    #[test]
    fn test_string_escapes() {
        let src = r#""hello\nworld\t""#;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::StringLiteral("hello\nworld\t".to_string()));
    }

    #[test]
    fn test_char_literal() {
        let src = "'a' '\\n' '\\\\'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::CharLiteral('a'));
        assert_eq!(tokens[1].token, Token::CharLiteral('\n'));
        assert_eq!(tokens[2].token, Token::CharLiteral('\\'));
    }

    #[test]
    fn test_comments() {
        let src = "42 // comment\n43 /* block */ 44";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let ints: Vec<&Token> = tokens.iter()
            .map(|st| &st.token)
            .filter(|t| matches!(t, Token::IntLiteral(_)))
            .collect();
        assert_eq!(ints, vec![&Token::IntLiteral(42), &Token::IntLiteral(43), &Token::IntLiteral(44)]);
    }

    #[test]
    fn test_operators() {
        let src = "== != <= >= << >> && || => -> :: &mut";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let ops: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(ops.contains(&&Token::EqualEqual));
        assert!(ops.contains(&&Token::NotEqual));
        assert!(ops.contains(&&Token::LessEqual));
        assert!(ops.contains(&&Token::GreaterEqual));
        assert!(ops.contains(&&Token::LeftShift));
        assert!(ops.contains(&&Token::RightShift));
        assert!(ops.contains(&&Token::AndAnd));
        assert!(ops.contains(&&Token::OrOr));
        assert!(ops.contains(&&Token::FatArrow));
        assert!(ops.contains(&&Token::Arrow));
        assert!(ops.contains(&&Token::ColonColon));
        assert!(ops.contains(&&Token::RefMut));
    }

    #[test]
    fn test_all_type_keywords() {
        let src = "void bool byte short int long vast uvast float double half quad char string size u8 u16 u32 u64";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let expected = vec![
            Token::Void, Token::Bool, Token::Byte, Token::Short,
            Token::Int, Token::Long, Token::Vast, Token::Uvast,
            Token::Float, Token::Double, Token::Half, Token::Quad,
            Token::Char, Token::String, Token::Size,
            Token::U8, Token::U16, Token::U32, Token::U64,
        ];
        let actual: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(actual[i], exp, "Mismatch at position {}", i);
        }
    }

    #[test]
    fn test_error_on_unterminated_string() {
        let src = "\"hello";
        let result = tokenize(src);
        assert!(result.is_err());
    }

    #[test]
    fn test_unrecognised_char() {
        let src = "@";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert!(tokens.iter().any(|st| matches!(st.token, Token::Error(_))));
    }

    #[test]
    fn test_operator_identifier() {
        let src = "fn operator+(self, other: Vec2): Vec2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(token_kinds.contains(&&Token::Identifier("operator+".to_string())),
            "Expected operator+ identifier, got: {:?}", token_kinds);
    }

    // -----------------------------------------------------------------------
    // Raw string literal tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_raw_string_simple() {
        let src = r##"r"hello world""##;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("hello world".to_string()));
    }

    #[test]
    fn test_raw_string_with_hash() {
        let src = r###"r#"hello "world""#"###;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("hello \"world\"".to_string()));
    }

    #[test]
    fn test_raw_string_no_escapes() {
        let src = r##"r"hello\nworld""##;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("hello\\nworld".to_string()));
    }

    #[test]
    fn test_raw_string_double_hash() {
        let src = r####"r##"contains #"# hash"##"####;
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("contains #\"# hash".to_string()));
    }

    #[test]
    fn test_raw_string_empty() {
        let src = "r\"\"";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::RawStringLiteral("".to_string()));
    }

    #[test]
    fn test_raw_string_vs_identifier() {
        let src = "return result";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::Return);
        assert_eq!(tokens[1].token, Token::Identifier("result".to_string()));
    }

    // -----------------------------------------------------------------------
    // Byte literal tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_byte_literal_simple() {
        let src = "b'x'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(b'x'));
    }

    #[test]
    fn test_byte_literal_escape() {
        let src = "b'\\n'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(b'\n'));
    }

    #[test]
    fn test_byte_literal_backslash() {
        let src = "b'\\\\'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(b'\\'));
    }

    #[test]
    fn test_byte_literal_hex_escape() {
        let src = "b'\\x41'";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::ByteLiteral(0x41));
    }

    #[test]
    fn test_byte_literal_vs_identifier() {
        let src = "bool byte";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::Bool);
        assert_eq!(tokens[1].token, Token::Byte);
    }

    // -----------------------------------------------------------------------
    // Underscore in numeric literals tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_underscore_decimal() {
        let src = "1_000_000";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(1000000));
    }

    #[test]
    fn test_underscore_hex() {
        let src = "0xFF_FF";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(0xFFFF));
    }

    #[test]
    fn test_underscore_binary() {
        let src = "0b1010_1100";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(0b10101100));
    }

    #[test]
    fn test_underscore_octal() {
        let src = "0o777_000";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(0o777000));
    }

    #[test]
    fn test_underscore_float() {
        let src = "1_000.5_0";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::FloatLiteral { value: 1000.50, suffix: None });
    }

    // -----------------------------------------------------------------------
    // Binary and octal literal tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_binary_literal() {
        let src = "0b1010";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(10));
    }

    #[test]
    fn test_binary_literal_uppercase() {
        let src = "0B1100";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(12));
    }

    #[test]
    fn test_octal_literal() {
        let src = "0o777";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(511));
    }

    #[test]
    fn test_octal_literal_uppercase() {
        let src = "0O123";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::IntLiteral(83));
    }

    // -----------------------------------------------------------------------
    // Range token tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_dot_dot() {
        let src = "1..10";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(token_kinds.contains(&&Token::DotDot), "Expected DotDot token, got: {:?}", token_kinds);
        assert_eq!(token_kinds[0], &Token::IntLiteral(1));
        assert_eq!(token_kinds[1], &Token::DotDot);
        assert_eq!(token_kinds[2], &Token::IntLiteral(10));
    }

    #[test]
    fn test_dot_dot_eq() {
        let src = "1..=10";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(token_kinds.contains(&&Token::DotDotEq), "Expected DotDotEq token, got: {:?}", token_kinds);
        assert_eq!(token_kinds[0], &Token::IntLiteral(1));
        assert_eq!(token_kinds[1], &Token::DotDotEq);
        assert_eq!(token_kinds[2], &Token::IntLiteral(10));
    }

    #[test]
    fn test_dot_still_works() {
        let src = "obj.field";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::Dot);
    }

    #[test]
    fn test_float_not_range() {
        let src = "1.5";
        let tokens = tokenize(src).expect("tokenize should succeed");
        assert_eq!(tokens[0].token, Token::FloatLiteral { value: 1.5, suffix: None });
    }

    // -----------------------------------------------------------------------
    // Compound assignment operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_compound_assignment_plus_equal() {
        let src = "x += 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[1], &Token::PlusEqual);
        assert_eq!(token_kinds[2], &Token::IntLiteral(1));
    }

    #[test]
    fn test_compound_assignment_minus_equal() {
        let src = "x -= 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::MinusEqual);
    }

    #[test]
    fn test_compound_assignment_star_equal() {
        let src = "x *= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::StarEqual);
    }

    #[test]
    fn test_compound_assignment_slash_equal() {
        let src = "x /= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::SlashEqual);
    }

    #[test]
    fn test_compound_assignment_percent_equal() {
        let src = "x %= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::PercentEqual);
    }

    #[test]
    fn test_compound_assignment_ampersand_equal() {
        let src = "x &= 3";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::AmpersandEqual);
    }

    #[test]
    fn test_compound_assignment_pipe_equal() {
        let src = "x |= 3";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::PipeEqual);
    }

    #[test]
    fn test_compound_assignment_caret_equal() {
        let src = "x ^= 3";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::CaretEqual);
    }

    #[test]
    fn test_compound_assignment_left_shift_equal() {
        let src = "x <<= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::LeftShiftEqual);
    }

    #[test]
    fn test_compound_assignment_right_shift_equal() {
        let src = "x >>= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::RightShiftEqual);
    }

    #[test]
    fn test_compound_assignment_vs_regular() {
        let src = "x += 1 y + 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::PlusEqual);
        assert_eq!(token_kinds[4], &Token::Plus);
    }

    #[test]
    fn test_left_shift_vs_less_equal() {
        let src = "x <<= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::LeftShiftEqual);
    }

    #[test]
    fn test_less_equal_not_shift_equal() {
        let src = "x <= 2";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[1], &Token::LessEqual);
    }

    #[test]
    fn test_all_compound_assignments() {
        let src = "+= -= *= /= %= &= |= ^= <<= >>=";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let ops: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert!(ops.contains(&&Token::PlusEqual));
        assert!(ops.contains(&&Token::MinusEqual));
        assert!(ops.contains(&&Token::StarEqual));
        assert!(ops.contains(&&Token::SlashEqual));
        assert!(ops.contains(&&Token::PercentEqual));
        assert!(ops.contains(&&Token::AmpersandEqual));
        assert!(ops.contains(&&Token::PipeEqual));
        assert!(ops.contains(&&Token::CaretEqual));
        assert!(ops.contains(&&Token::LeftShiftEqual));
        assert!(ops.contains(&&Token::RightShiftEqual));
    }

    // -----------------------------------------------------------------------
    // Increment/decrement operator tests
    // -----------------------------------------------------------------------
    #[test]
    fn test_plus_plus() {
        let src = "++x";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::PlusPlus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
    }

    #[test]
    fn test_minus_minus() {
        let src = "--x";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::MinusMinus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
    }

    #[test]
    fn test_postfix_plus_plus() {
        let src = "x++";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[1], &Token::PlusPlus);
    }

    #[test]
    fn test_postfix_minus_minus() {
        let src = "x--";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[1], &Token::MinusMinus);
    }

    #[test]
    fn test_plus_plus_vs_plus_equal() {
        let src = "++x x++ x += 1";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::PlusPlus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[2], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[3], &Token::PlusPlus);
        assert_eq!(token_kinds[4], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[5], &Token::PlusEqual);
    }

    #[test]
    fn test_minus_minus_vs_arrow() {
        let src = "--x x-- x -> y";
        let tokens = tokenize(src).expect("tokenize should succeed");
        let token_kinds: Vec<&Token> = tokens.iter().map(|st| &st.token).collect();
        assert_eq!(token_kinds[0], &Token::MinusMinus);
        assert_eq!(token_kinds[1], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[2], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[3], &Token::MinusMinus);
        assert_eq!(token_kinds[4], &Token::Identifier("x".to_string()));
        assert_eq!(token_kinds[5], &Token::Arrow);
    }
