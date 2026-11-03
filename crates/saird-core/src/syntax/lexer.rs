//! Lines, indentation and tokens of contracts and assemblies
//! (docs/reference/syntax.md, "行と字下げ" and "字句").
//!
//! The source becomes a tree of logical lines. A physical line continues the
//! logical line before it while a bracket is open, or when it is indented
//! deeper than that logical line and that line does not end with ':'. A
//! logical line that ends with ':' owns the more deeply indented lines below
//! it, all at one depth.

use super::error::{Span, SyntaxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// A run of `[A-Za-z0-9_]`: names, keywords, numbers, durations, versions.
    Word(String),
    /// A string literal, with its escapes resolved.
    Str(String),
    /// The text of a `summary:` line at the top level.
    Text(String),
    Colon,
    Comma,
    Dot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Ne,
    Eq,
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) tok: Tok,
    pub(crate) span: Span,
    /// The width in the source, in characters.
    pub(crate) width: usize,
}

/// A logical line, and the block it owns when it ends with ':'.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) tokens: Vec<Token>,
    pub(crate) children: Vec<Line>,
    pub(crate) span: Span,
    /// Just after the last token.
    pub(crate) end: Span,
}

/// A logical line before the blocks are built.
struct Flat {
    indent: usize,
    tokens: Vec<Token>,
    span: Span,
}

/// How a token is shown in an error message.
pub(crate) fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Word(w) => format!("'{w}'"),
        Tok::Str(_) => "a string".to_string(),
        Tok::Text(_) => "text".to_string(),
        other => format!("'{}'", symbol(other)),
    }
}

fn symbol(tok: &Tok) -> &'static str {
    match tok {
        Tok::Colon => ":",
        Tok::Comma => ",",
        Tok::Dot => ".",
        Tok::LParen => "(",
        Tok::RParen => ")",
        Tok::LBracket => "[",
        Tok::RBracket => "]",
        Tok::LBrace => "{",
        Tok::RBrace => "}",
        Tok::Lt => "<",
        Tok::Gt => ">",
        Tok::Le => "<=",
        Tok::Ge => ">=",
        Tok::EqEq => "==",
        Tok::Ne => "!=",
        Tok::Eq => "=",
        Tok::Plus => "+",
        Tok::Minus => "-",
        Tok::Word(_) | Tok::Str(_) | Tok::Text(_) => "",
    }
}

/// Splits a source into the tree of its logical lines.
pub(crate) fn lines(source: &str) -> Result<Vec<Line>, SyntaxError> {
    let mut flat: Vec<Flat> = Vec::new();
    let mut open: Vec<Token> = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let indent = raw.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        // The indentation is spaces and tabs only, one byte each.
        let rest = raw.get(indent..).unwrap_or_default();
        let continues = flat.last().is_some_and(|prev| {
            !open.is_empty() || (indent > prev.indent && !ends_with_colon(&prev.tokens))
        });
        let mut tokens = Vec::new();
        match summary(rest, line, indent) {
            Some(summary) if !continues => tokens = summary,
            _ => tokenize(rest, line, indent + 1, &mut tokens)?,
        }
        if tokens.is_empty() {
            continue;
        }
        if let Some(column) = raw.chars().take(indent).position(|c| c == '\t') {
            return Err(SyntaxError::new(
                Span {
                    line,
                    column: column + 1,
                },
                "a tab in the indentation (indent with spaces)",
            ));
        }
        match flat.last_mut() {
            Some(prev) if continues => {
                let before = prev.tokens.last().map(|t| t.tok.clone());
                track(&tokens, before, &mut open)?;
                prev.tokens.extend(tokens);
            }
            _ => {
                track(&tokens, None, &mut open)?;
                flat.push(Flat {
                    indent,
                    tokens,
                    span: Span {
                        line,
                        column: indent + 1,
                    },
                });
            }
        }
    }
    if let Some(unclosed) = open.last() {
        return Err(SyntaxError::new(
            unclosed.span,
            format!("{} is not closed", describe(&unclosed.tok)),
        ));
    }
    let mut pos = 0;
    block(&flat, &mut pos, 0)
}

fn ends_with_colon(tokens: &[Token]) -> bool {
    tokens.last().is_some_and(|t| t.tok == Tok::Colon)
}

fn end_of(flat: &Flat) -> Span {
    flat.tokens.last().map_or(flat.span, |t| Span {
        line: t.span.line,
        column: t.span.column + t.width,
    })
}

/// Builds the lines indented by `indent`, and the blocks below them.
fn block(flat: &[Flat], pos: &mut usize, indent: usize) -> Result<Vec<Line>, SyntaxError> {
    let mut out = Vec::new();
    while let Some(line) = flat.get(*pos) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(SyntaxError::new(line.span, "unexpected indentation"));
        }
        *pos += 1;
        let mut children = Vec::new();
        if ends_with_colon(&line.tokens) {
            match flat.get(*pos) {
                Some(next) if next.indent > indent => children = block(flat, pos, next.indent)?,
                _ => {
                    return Err(SyntaxError::new(
                        end_of(line),
                        "expected an indented block after ':'",
                    ));
                }
            }
        }
        out.push(Line {
            tokens: line.tokens.clone(),
            children,
            span: line.span,
            end: end_of(line),
        });
    }
    Ok(out)
}

/// Keeps the brackets that are open. `<` opens one only after `list`;
/// elsewhere it compares, and so does a `>` that closes nothing.
fn track(tokens: &[Token], before: Option<Tok>, open: &mut Vec<Token>) -> Result<(), SyntaxError> {
    let mut before = before.as_ref();
    for token in tokens {
        let closes = match token.tok {
            Tok::RParen => Some(Tok::LParen),
            Tok::RBracket => Some(Tok::LBracket),
            Tok::RBrace => Some(Tok::LBrace),
            _ => None,
        };
        match &token.tok {
            Tok::LParen | Tok::LBracket | Tok::LBrace => open.push(token.clone()),
            Tok::Lt if matches!(before, Some(Tok::Word(w)) if w == "list") => {
                open.push(token.clone());
            }
            Tok::Gt if open.last().is_some_and(|o| o.tok == Tok::Lt) => {
                open.pop();
            }
            _ => {}
        }
        if let Some(want) = closes {
            match open.pop() {
                Some(o) if o.tok == want => {}
                Some(o) => {
                    return Err(SyntaxError::new(
                        token.span,
                        format!(
                            "{} does not close {} at {}",
                            describe(&token.tok),
                            describe(&o.tok),
                            o.span
                        ),
                    ));
                }
                None => {
                    return Err(SyntaxError::new(
                        token.span,
                        format!("{} closes nothing", describe(&token.tok)),
                    ));
                }
            }
        }
        before = Some(&token.tok);
    }
    Ok(())
}

/// The tokens of a top-level `summary:` line: the rest of the line after ':'
/// is its text, up to a comment.
fn summary(rest: &str, line: usize, indent: usize) -> Option<Vec<Token>> {
    if indent != 0 {
        return None;
    }
    let after = rest.strip_prefix("summary")?;
    let gap = after.len() - after.trim_start_matches([' ', '\t']).len();
    let text = after.get(gap..)?.strip_prefix(':')?;
    let text = text.split('#').next().unwrap_or_default();
    let lead = text.chars().take_while(|c| c.is_whitespace()).count();
    let trimmed = text.trim();
    let colon = 1 + "summary".len() + gap;
    Some(vec![
        Token {
            tok: Tok::Word("summary".to_string()),
            span: Span { line, column: 1 },
            width: "summary".len(),
        },
        Token {
            tok: Tok::Colon,
            span: Span {
                line,
                column: colon,
            },
            width: 1,
        },
        Token {
            tok: Tok::Text(trimmed.to_string()),
            span: Span {
                line,
                column: colon + 1 + lead,
            },
            width: trimmed.chars().count(),
        },
    ])
}

/// Adds the tokens of `text` (a line after its indentation, which starts at
/// `column`) to `out`.
fn tokenize(
    text: &str,
    line: usize,
    column: usize,
    out: &mut Vec<Token>,
) -> Result<(), SyntaxError> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        let span = Span {
            line,
            column: column + i,
        };
        let start = i;
        let tok = match c {
            ' ' | '\t' => {
                i += 1;
                continue;
            }
            '#' => break,
            '"' => {
                let (value, next) = string(&chars, i, line, column)?;
                i = next;
                Tok::Str(value)
            }
            c if c.is_ascii_alphanumeric() || c == '_' => {
                while chars
                    .get(i)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                {
                    i += 1;
                }
                Tok::Word(chars.get(start..i).unwrap_or_default().iter().collect())
            }
            _ => {
                let (tok, width) = match (c, chars.get(i + 1)) {
                    ('<', Some('=')) => (Tok::Le, 2),
                    ('>', Some('=')) => (Tok::Ge, 2),
                    ('=', Some('=')) => (Tok::EqEq, 2),
                    ('!', Some('=')) => (Tok::Ne, 2),
                    ('<', _) => (Tok::Lt, 1),
                    ('>', _) => (Tok::Gt, 1),
                    ('=', _) => (Tok::Eq, 1),
                    (':', _) => (Tok::Colon, 1),
                    (',', _) => (Tok::Comma, 1),
                    ('.', _) => (Tok::Dot, 1),
                    ('(', _) => (Tok::LParen, 1),
                    (')', _) => (Tok::RParen, 1),
                    ('[', _) => (Tok::LBracket, 1),
                    (']', _) => (Tok::RBracket, 1),
                    ('{', _) => (Tok::LBrace, 1),
                    ('}', _) => (Tok::RBrace, 1),
                    ('+', _) => (Tok::Plus, 1),
                    ('-', _) => (Tok::Minus, 1),
                    _ => {
                        return Err(SyntaxError::new(
                            span,
                            format!("unexpected character {c:?}"),
                        ));
                    }
                };
                i += width;
                tok
            }
        };
        out.push(Token {
            tok,
            span,
            width: i - start,
        });
    }
    Ok(())
}

/// Reads the string that opens at `chars[open]`. Returns its value and the
/// index after its closing quote. Only `\"` and `\\` are escapes.
fn string(
    chars: &[char],
    open: usize,
    line: usize,
    column: usize,
) -> Result<(String, usize), SyntaxError> {
    let mut value = String::new();
    let mut i = open + 1;
    loop {
        match chars.get(i) {
            None => {
                return Err(SyntaxError::new(
                    Span {
                        line,
                        column: column + open,
                    },
                    "the string is not closed on its line",
                ));
            }
            Some('"') => return Ok((value, i + 1)),
            Some('\\') => match chars.get(i + 1) {
                Some(&c @ ('"' | '\\')) => {
                    value.push(c);
                    i += 2;
                }
                _ => {
                    return Err(SyntaxError::new(
                        Span {
                            line,
                            column: column + i,
                        },
                        "unknown escape in a string (only \\\" and \\\\ are escapes)",
                    ));
                }
            },
            Some(&c) => {
                value.push(c);
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree of logical lines, one line of text each, indented two spaces
    /// per depth. Strings are quoted, summary text is in <>.
    fn render(source: &str) -> Vec<String> {
        fn walk(lines: &[Line], depth: usize, out: &mut Vec<String>) {
            for line in lines {
                let words: Vec<String> = line
                    .tokens
                    .iter()
                    .map(|t| match &t.tok {
                        Tok::Word(w) => w.clone(),
                        Tok::Str(s) => format!("{s:?}"),
                        Tok::Text(s) => format!("<{s}>"),
                        other => symbol(other).to_string(),
                    })
                    .collect();
                out.push(format!("{}{}", "  ".repeat(depth), words.join(" ")));
                walk(&line.children, depth + 1, out);
            }
        }
        let mut out = Vec::new();
        walk(&lines(source).expect("the source lexes"), 0, &mut out);
        out
    }

    fn error(source: &str) -> String {
        lines(source)
            .expect_err("the source is refused")
            .to_string()
    }

    #[test]
    fn blocks_and_continued_lines() {
        let source = "in:\n  items: list<{ a: int,\n                b: int }>\n         where len(items) <= 3\n  n: int\nout:\n  ok: int\n";
        assert_eq!(
            render(source),
            [
                "in :",
                "  items : list < { a : int , b : int } > where len ( items ) <= 3",
                "  n : int",
                "out :",
                "  ok : int",
            ]
        );
    }

    #[test]
    fn a_deeper_line_after_a_colon_starts_a_block() {
        assert_eq!(render("a:\n  b\n  c\n"), ["a :", "  b", "  c"]);
    }

    #[test]
    fn a_comparing_lt_opens_no_bracket() {
        // Were "<" a bracket, "b" would continue the line of "a".
        assert_eq!(
            render("in:\n  a: int where 1 < a\n  b: int\n"),
            ["in :", "  a : int where 1 < a", "  b : int"]
        );
        assert_eq!(
            render("x: list<int> > 1\ny\n"),
            ["x : list < int > > 1", "y"]
        );
        // A comparing ">" closes nothing, not even an open '('.
        assert_eq!(render("x: (a > b)\n"), ["x : ( a > b )"]);
    }

    #[test]
    fn the_lt_of_a_list_keeps_the_line_open() {
        // Only the open '<' carries "int>" over: it is not indented deeper.
        assert_eq!(
            render("in:\n  x: list<\n  int>\n"),
            ["in :", "  x : list < int >"]
        );
    }

    #[test]
    fn comments_and_blank_lines() {
        assert_eq!(
            render("# head\n\nx: \"a # b\"  # note\n   # indented comment\ny\n"),
            ["x : \"a # b\"", "y"]
        );
    }

    #[test]
    fn summary_text_up_to_a_comment() {
        assert_eq!(
            render("summary:  注文の在庫を引き当てる  # memo\n"),
            ["summary : <注文の在庫を引き当てる>"]
        );
        let lines = lines("summary:  注文\n").expect("the source lexes");
        let text = lines
            .first()
            .and_then(|l| l.tokens.get(2))
            .expect("a text token");
        assert_eq!(
            text.span,
            Span {
                line: 1,
                column: 11
            }
        );
    }

    #[test]
    fn summary_only_at_the_top_level() {
        assert_eq!(render("in:\n  summary: int\n"), ["in :", "  summary : int"]);
        // Nor on a line that continues another inside a bracket.
        assert_eq!(render("x: (a,\nsummary: b)\n"), ["x : ( a , summary : b )"]);
    }

    #[test]
    fn summary_with_a_space_before_the_colon() {
        let lines = lines("summary :  x\n").expect("the source lexes");
        let spans: Vec<usize> = lines
            .first()
            .expect("a line")
            .tokens
            .iter()
            .map(|t| t.span.column)
            .collect();
        assert_eq!(spans, [1, 9, 12]);
    }

    #[test]
    fn string_escapes() {
        assert_eq!(render(r#"x "a\"b\\c""#), [r#"x "a\"b\\c""#]);
    }

    #[test]
    fn spans_count_characters() {
        let lines = lines("  \n\"日本\" x\n").expect("the source lexes");
        let tokens = &lines.first().expect("a line").tokens;
        let spans: Vec<(usize, usize, usize)> = tokens
            .iter()
            .map(|t| (t.span.line, t.span.column, t.width))
            .collect();
        assert_eq!(spans, [(2, 1, 4), (2, 6, 1)]);
    }

    #[test]
    fn refused_sources() {
        let cases = [
            (
                "in:\n\tx: int\n",
                "line 2, column 1: a tab in the indentation (indent with spaces)",
            ),
            (
                "in:\n  \tx: int\n",
                "line 2, column 3: a tab in the indentation (indent with spaces)",
            ),
            (
                "x: \"abc\n",
                "line 1, column 4: the string is not closed on its line",
            ),
            (
                "x: \"a\\nb\"\n",
                "line 1, column 6: unknown escape in a string (only \\\" and \\\\ are escapes)",
            ),
            (
                "in:\nout:\n",
                "line 1, column 4: expected an indented block after ':'",
            ),
            (
                "a:\n    b\n  c\n",
                "line 3, column 3: unexpected indentation",
            ),
            ("  box\n", "line 1, column 3: unexpected indentation"),
            ("x: {a\n", "line 1, column 4: '{' is not closed"),
            (
                "x: (a]\n",
                "line 1, column 6: ']' does not close '(' at line 1, column 4",
            ),
            ("x: a)\n", "line 1, column 5: ')' closes nothing"),
            ("x: @\n", "line 1, column 4: unexpected character '@'"),
            ("x: a ! b\n", "line 1, column 6: unexpected character '!'"),
        ];
        for (source, want) in cases {
            assert_eq!(error(source), want, "source: {source:?}");
        }
    }
}
