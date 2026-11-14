use mithqal::diag::ErrorCode;
use mithqal::syntax::{TokenKind, lex};

fn kinds(source: &str) -> Vec<TokenKind> {
    lex(source)
        .unwrap()
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Newline)
        .collect()
}

#[test]
fn lexes_numbers_and_identifiers() {
    assert_eq!(kinds("0"), vec![TokenKind::Number("0".into())]);
    assert_eq!(kinds("1.25"), vec![TokenKind::Number("1.25".into())]);
    assert_eq!(kinds("fooBar1"), vec![TokenKind::Ident("fooBar1".into())]);
    assert_eq!(
        kinds("1e4"),
        vec![TokenKind::Number("1".into()), TokenKind::Ident("e4".into())]
    );
}

#[test]
fn lexes_pattern_suffixes() {
    assert_eq!(kinds("_"), vec![TokenKind::Underscore]);
    assert_eq!(kinds("__"), vec![TokenKind::Underscore2]);
    assert_eq!(kinds("___"), vec![TokenKind::Underscore3]);
    assert_eq!(
        kinds("x_"),
        vec![TokenKind::Ident("x".into()), TokenKind::Underscore]
    );
    assert_eq!(
        kinds("x___"),
        vec![TokenKind::Ident("x".into()), TokenKind::Underscore3]
    );
}

#[test]
fn lexes_all_operators() {
    let cases = [
        ("+", TokenKind::Plus),
        ("-", TokenKind::Minus),
        ("*", TokenKind::Star),
        ("/", TokenKind::Slash),
        ("^", TokenKind::Caret),
        ("(", TokenKind::LParen),
        (")", TokenKind::RParen),
        ("[", TokenKind::LBracket),
        ("]", TokenKind::RBracket),
        (",", TokenKind::Comma),
        ("_", TokenKind::Underscore),
        ("=", TokenKind::Equal),
        (":=", TokenKind::SetDelayed),
        ("->", TokenKind::Arrow),
        (":>", TokenKind::RuleDelayed),
        ("=>", TokenKind::FatArrow),
        ("===", TokenKind::Equal3),
        ("=/=", TokenKind::NotEqual),
        ("~", TokenKind::Tilde),
        ("<", TokenKind::Less),
        ("<=", TokenKind::LessEqual),
        (">", TokenKind::Greater),
        (">=", TokenKind::GreaterEqual),
        ("&&", TokenKind::And),
        ("||", TokenKind::Or),
        ("!", TokenKind::Not),
    ];
    for (source, expected) in cases {
        assert_eq!(kinds(source), vec![expected], "operator {source}");
    }
}

#[test]
fn newlines_are_statement_separators() {
    let tokens = lex("1\n2\n").unwrap();
    assert_eq!(
        tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Newline)
            .count(),
        2
    );
}

#[test]
fn blank_lines_and_indented_blank_lines_are_allowed() {
    assert!(lex("\n").is_ok());
    assert!(lex("   \n\t\n").is_ok());
}

#[test]
fn comments_are_ignored() {
    assert_eq!(kinds("(* comment *)"), Vec::<TokenKind>::new());
    assert_eq!(
        kinds("(* first\nsecond *)\n1"),
        vec![TokenKind::Number("1".into())]
    );
    assert_eq!(
        kinds("(* outer (* inner *) outer *)\n1"),
        vec![TokenKind::Number("1".into())]
    );
}

#[test]
fn comments_may_be_indented() {
    assert!(lex("  (* indented *)\n1+2\n").is_ok());
}

#[test]
fn inline_comments_are_errors() {
    let error = lex("1+2(* comment *)").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseInlineComment);
    let error = lex("(* comment *)1+2").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseInlineComment);
}

#[test]
fn unclosed_comment_is_an_error() {
    let error = lex("(* never closed\nstill open\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnclosedComment);
}

#[test]
fn whitespace_inside_a_statement_is_an_error() {
    for source in ["1 2", "1\t2", "1\r2", " 1", "1 ", "a b"] {
        let error = lex(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseWhitespace, "source {source:?}");
    }
}

#[test]
fn invalid_number_forms_are_errors() {
    for source in ["1.", "1..2", "1.2.3"] {
        let error = lex(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseInvalidNumber, "source {source}");
    }
}

#[test]
fn too_many_pattern_underscores_is_an_error() {
    let error = lex("____").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedCharacter);
}

#[test]
fn invalid_operators_are_errors() {
    for source in ["&", "|", ":", "@", "$"] {
        let error = lex(source).unwrap_err();
        assert_eq!(
            error.code,
            ErrorCode::ParseUnexpectedCharacter,
            "source {source}"
        );
    }
}

#[test]
fn non_ascii_source_is_an_error() {
    let error = lex("π").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedCharacter);
}

#[test]
fn spans_report_line_and_column() {
    let error = lex("1+2\n3 4\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseWhitespace);
    let span = error.span.unwrap();
    assert_eq!(span.line, 2);
    assert_eq!(span.column, 2);
}

#[test]
fn comment_lines_allow_trailing_spaces() {
    assert!(lex("(* comment *)   \n1\n").is_ok());
}

#[test]
fn code_cannot_follow_a_comment_on_the_same_line() {
    let error = lex("(* comment *)  1\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseInlineComment);
}
