//! The parser of contracts (docs/reference/syntax.md, "契約" and "式").

use super::ast::{
    Access, CmpOp, Contract, Duration, DurationUnit, Effect, ErrorDecl, ErrorField, Example,
    Expect, Expr, ExprKind, Field, Generate, Given, KvMap, KvNeed, KvState, Need, Outcome, Output,
    Path, Test, Type, Value, ValueKind,
};
use super::error::{Span, SyntaxError};
use super::lexer::{self, Line, Tok, Token, describe};

/// Words with a meaning of their own in expressions; they cannot start a path.
const RESERVED: &[&str] = &[
    "and",
    "any",
    "each",
    "false",
    "in",
    "kv_after",
    "kv_before",
    "len",
    "matches",
    "not",
    "or",
    "true",
    "unique",
];

/// Parses a box contract. Only the grammar is checked: whether the contract
/// makes sense (bounds, names, types) is for the checker.
pub fn parse_contract(source: &str) -> Result<Contract, SyntaxError> {
    let lines = lexer::lines(source)?;
    let eof = Span {
        line: source.lines().count() + 1,
        column: 1,
    };
    const END: &str = "the end of the input";
    let mut rest = lines.iter();

    let head = next_line(&mut rest, eof, END, "'box'")?;
    let (name, version) = whole(head, |c| {
        c.keyword("box")?;
        let (name, _) = c.name("the name of the box")?;
        let version = c.version()?;
        Ok((name, version))
    })?;
    let summary = summary(next_line(&mut rest, eof, END, "'summary:'")?)?;
    let inputs = section(next_line(&mut rest, eof, END, "'in:'")?, "in")?
        .iter()
        .map(|line| whole(line, field))
        .collect::<Result<_, _>>()?;
    let output = output(next_line(&mut rest, eof, END, "'out:'")?)?;
    // needs: may be left out: a box with no permission (pure computation).
    let needs = match rest.as_slice().first() {
        Some(line) if starts_with(line, "needs") => {
            rest.next();
            section(line, "needs")?
                .iter()
                .map(|line| whole(line, need))
                .collect::<Result<_, _>>()?
        }
        _ => Vec::new(),
    };
    let expected = if needs.is_empty() {
        "'needs:' or 'tests:'"
    } else {
        "'tests:'"
    };
    let tests = section(next_line(&mut rest, eof, END, expected)?, "tests")?
        .iter()
        .map(test)
        .collect::<Result<_, _>>()?;
    if let Some(line) = rest.next() {
        return Err(SyntaxError::new(
            line.span,
            "unexpected line after 'tests:'",
        ));
    }
    Ok(Contract {
        name,
        version,
        summary,
        inputs,
        output,
        needs,
        tests,
    })
}

/// The tokens of one logical line, read from the left.
struct Cursor<'a> {
    tokens: &'a [Token],
    pos: usize,
    end: Span,
}

impl<'a> Cursor<'a> {
    fn new(line: &'a Line) -> Self {
        Self {
            tokens: &line.tokens,
            pos: 0,
            end: line.end,
        }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, n: usize) -> Option<&'a Token> {
        self.tokens.get(self.pos + n)
    }

    fn peek_tok(&self) -> Option<&'a Tok> {
        self.peek().map(|t| &t.tok)
    }

    /// Where the next token is, or the end of the line.
    fn here(&self) -> Span {
        self.peek().map_or(self.end, |t| t.span)
    }

    /// Moves past the next token. Past the end, `peek` keeps finding nothing.
    fn advance(&mut self) {
        self.pos += 1;
    }

    fn at(&self, tok: &Tok) -> bool {
        self.peek_tok() == Some(tok)
    }

    fn at_word(&self, word: &str) -> bool {
        matches!(self.peek_tok(), Some(Tok::Word(w)) if w == word)
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        let hit = self.at(tok);
        if hit {
            self.advance();
        }
        hit
    }

    fn eat_word(&mut self, word: &str) -> bool {
        let hit = self.at_word(word);
        if hit {
            self.advance();
        }
        hit
    }

    fn expect(&mut self, tok: &Tok, what: &str) -> Result<Span, SyntaxError> {
        let span = self.here();
        if self.eat(tok) {
            Ok(span)
        } else {
            Err(self.unexpected(what))
        }
    }

    fn keyword(&mut self, word: &str) -> Result<Span, SyntaxError> {
        let span = self.here();
        if self.eat_word(word) {
            Ok(span)
        } else {
            Err(self.unexpected(&format!("'{word}'")))
        }
    }

    fn word_where(
        &mut self,
        what: &str,
        ok: impl Fn(&str) -> bool,
    ) -> Result<(String, Span), SyntaxError> {
        match self.peek() {
            Some(Token {
                tok: Tok::Word(w),
                span,
                ..
            }) if ok(w) => {
                self.advance();
                Ok((w.clone(), *span))
            }
            _ => Err(self.unexpected(what)),
        }
    }

    fn ident(&mut self, what: &str) -> Result<(String, Span), SyntaxError> {
        self.word_where(what, is_ident)
    }

    /// An identifier that is not reserved: a variable, or the start of a path.
    fn variable(&mut self, what: &str) -> Result<(String, Span), SyntaxError> {
        self.word_where(what, |w| is_ident(w) && !RESERVED.contains(&w))
    }

    fn name(&mut self, what: &str) -> Result<(String, Span), SyntaxError> {
        self.word_where(what, is_name)
    }

    fn string(&mut self, what: &str) -> Result<(String, Span), SyntaxError> {
        match self.peek() {
            Some(Token {
                tok: Tok::Str(s),
                span,
                ..
            }) => {
                self.advance();
                Ok((s.clone(), *span))
            }
            _ => Err(self.unexpected(what)),
        }
    }

    fn version(&mut self) -> Result<u32, SyntaxError> {
        if let Some(Tok::Word(w)) = self.peek_tok()
            && let Some(version) = parse_version(w)
        {
            self.advance();
            return Ok(version);
        }
        Err(self.unexpected("a version (v1, v2, ...)"))
    }

    /// A non-negative integer, such as the count of runs.
    fn count(&mut self) -> Result<u64, SyntaxError> {
        let span = self.here();
        if let Some(Tok::Word(w)) = self.peek_tok()
            && is_digits(w)
        {
            self.advance();
            return w
                .parse()
                .map_err(|_| SyntaxError::new(span, "the number is too large"));
        }
        Err(self.unexpected("a number"))
    }

    fn unexpected(&self, expected: &str) -> SyntaxError {
        match self.peek() {
            Some(t) => SyntaxError::new(
                t.span,
                format!("expected {expected}, found {}", describe(&t.tok)),
            ),
            None => SyntaxError::new(
                self.end,
                format!("expected {expected}, found the end of the line"),
            ),
        }
    }

    fn finish(&self) -> Result<(), SyntaxError> {
        match self.peek() {
            Some(_) => Err(self.unexpected("the end of the line")),
            None => Ok(()),
        }
    }
}

fn is_ident(w: &str) -> bool {
    let mut chars = w.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_name(w: &str) -> bool {
    let mut chars = w.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_alphanumeric())
}

fn is_digits(w: &str) -> bool {
    !w.is_empty() && w.bytes().all(|b| b.is_ascii_digit())
}

fn parse_version(w: &str) -> Option<u32> {
    w.strip_prefix('v')
        .filter(|digits| is_digits(digits))
        .and_then(|digits| digits.parse().ok())
}

fn parse_duration(w: &str) -> Option<Duration> {
    let digits = w.len() - w.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let (amount, unit) = w.split_at(digits);
    if amount.is_empty() {
        return None;
    }
    let unit = match unit {
        "s" => DurationUnit::Seconds,
        "min" => DurationUnit::Minutes,
        "h" => DurationUnit::Hours,
        "d" => DurationUnit::Days,
        _ => return None,
    };
    Some(Duration {
        amount: amount.parse().ok()?,
        unit,
    })
}

/// A 64-bit signed integer from its digits.
fn int(digits: &str, negative: bool, span: Span) -> Result<i64, SyntaxError> {
    let out_of_range = || SyntaxError::new(span, "the integer is out of the 64-bit range");
    let n: i128 = digits.parse().map_err(|_| out_of_range())?;
    i64::try_from(if negative { -n } else { n }).map_err(|_| out_of_range())
}

/// Whether `b` follows `a` with no space between them.
fn adjacent(a: &Token, b: &Token) -> bool {
    a.span.line == b.span.line && a.span.column + a.width == b.span.column
}

/// A '-' and the digits right after it, if the cursor is at them.
fn negative_digits<'a>(c: &Cursor<'a>) -> Option<&'a str> {
    match (c.peek(), c.peek_at(1)) {
        (Some(minus), Some(next)) if minus.tok == Tok::Minus && adjacent(minus, next) => {
            match &next.tok {
                Tok::Word(w) if is_digits(w) => Some(w),
                _ => None,
            }
        }
        _ => None,
    }
}

fn next_line<'a>(
    lines: &mut std::slice::Iter<'a, Line>,
    at: Span,
    nothing: &str,
    expected: &str,
) -> Result<&'a Line, SyntaxError> {
    lines
        .next()
        .ok_or_else(|| SyntaxError::new(at, format!("expected {expected}, found {nothing}")))
}

/// Parses a whole line with `f`; nothing may follow.
fn whole<T>(
    line: &Line,
    f: impl FnOnce(&mut Cursor<'_>) -> Result<T, SyntaxError>,
) -> Result<T, SyntaxError> {
    let mut c = Cursor::new(line);
    let value = f(&mut c)?;
    c.finish()?;
    Ok(value)
}

/// A line that is exactly `name:`, and the block it owns.
fn section<'a>(line: &'a Line, name: &str) -> Result<&'a [Line], SyntaxError> {
    whole(line, |c| {
        c.keyword(name)?;
        c.expect(&Tok::Colon, "':'")?;
        Ok(())
    })?;
    Ok(&line.children)
}

fn starts_with(line: &Line, word: &str) -> bool {
    matches!(line.tokens.first().map(|t| &t.tok), Some(Tok::Word(w)) if w == word)
}

fn summary(line: &Line) -> Result<String, SyntaxError> {
    whole(line, |c| {
        c.keyword("summary")?;
        c.expect(&Tok::Colon, "':'")?;
        match c.peek_tok() {
            Some(Tok::Text(text)) => {
                c.advance();
                Ok(text.clone())
            }
            _ => Err(c.unexpected("the summary")),
        }
    })
}

fn output(line: &Line) -> Result<Output, SyntaxError> {
    let children = section(line, "out")?;
    let mut rest = children.iter();
    let nothing = "the end of 'out:'";
    let ok_line = next_line(&mut rest, line.end, nothing, "'ok:'")?;
    let ok = whole(ok_line, |c| {
        c.keyword("ok")?;
        c.expect(&Tok::Colon, "':'")?;
        ty(c)
    })?;
    let ensures = section(
        next_line(&mut rest, line.end, nothing, "'ensures:'")?,
        "ensures",
    )?
    .iter()
    .map(|line| whole(line, expr))
    .collect::<Result<_, _>>()?;
    // errors: may be left out: a box that declares no error.
    let errors = match rest.next() {
        Some(errors) if starts_with(errors, "errors") => section(errors, "errors")?
            .iter()
            .map(|line| whole(line, error_decl))
            .collect::<Result<_, _>>()?,
        Some(extra) => return Err(Cursor::new(extra).unexpected("'errors:'")),
        None => Vec::new(),
    };
    if let Some(extra) = rest.next() {
        return Err(SyntaxError::new(
            extra.span,
            "unexpected line after 'errors:'",
        ));
    }
    Ok(Output {
        ok,
        ok_span: ok_line.span,
        ensures,
        errors,
    })
}

fn field(c: &mut Cursor<'_>) -> Result<Field, SyntaxError> {
    let (name, span) = c.ident("a field name")?;
    c.expect(&Tok::Colon, "':'")?;
    let ty = ty(c)?;
    let constraints = if c.eat_word("where") {
        constraints(c)?
    } else {
        Vec::new()
    };
    Ok(Field {
        name,
        ty,
        constraints,
        span,
    })
}

fn ty(c: &mut Cursor<'_>) -> Result<Type, SyntaxError> {
    if c.eat(&Tok::LBrace) {
        let mut fields = vec![field(c)?];
        while c.eat(&Tok::Comma) {
            fields.push(field(c)?);
        }
        c.expect(&Tok::RBrace, "',' or '}'")?;
        return Ok(Type::Record(fields));
    }
    let simple = match c.peek_tok() {
        Some(Tok::Word(w)) => match w.as_str() {
            "int" => Some(Type::Int),
            "string" => Some(Type::String),
            "bool" => Some(Type::Bool),
            "timestamp" => Some(Type::Timestamp),
            _ => None,
        },
        _ => None,
    };
    if let Some(simple) = simple {
        c.advance();
        return Ok(simple);
    }
    if c.eat_word("list") {
        c.expect(&Tok::Lt, "'<'")?;
        let inner = ty(c)?;
        c.expect(&Tok::Gt, "'>'")?;
        return Ok(Type::List(Box::new(inner)));
    }
    Err(c.unexpected("a type"))
}

/// The expressions after `where`. In a record type, ", name:" starts the next
/// field instead.
fn constraints(c: &mut Cursor<'_>) -> Result<Vec<Expr>, SyntaxError> {
    let mut out = vec![expr(c)?];
    while c.at(&Tok::Comma) {
        let next_field = matches!(
            (c.peek_at(1).map(|t| &t.tok), c.peek_at(2).map(|t| &t.tok)),
            (Some(Tok::Word(w)), Some(Tok::Colon)) if is_ident(w)
        );
        if next_field {
            break;
        }
        c.advance();
        out.push(expr(c)?);
    }
    Ok(out)
}

fn error_decl(c: &mut Cursor<'_>) -> Result<ErrorDecl, SyntaxError> {
    let (name, span) = c.name("the name of an error")?;
    let mut fields = Vec::new();
    if c.eat(&Tok::LBrace) {
        loop {
            let (name, span) = c.ident("a field name")?;
            let value = if c.eat(&Tok::Colon) {
                Some(expr(c)?)
            } else {
                None
            };
            fields.push(ErrorField { name, value, span });
            if !c.eat(&Tok::Comma) {
                break;
            }
        }
        c.expect(&Tok::RBrace, "',' or '}'")?;
    }
    c.keyword("when")?;
    let when = expr(c)?;
    Ok(ErrorDecl {
        name,
        fields,
        when,
        span,
    })
}

fn need(c: &mut Cursor<'_>) -> Result<Need, SyntaxError> {
    let span = c.here();
    if c.eat_word("kv") {
        let (template, _) = c.string("a key template")?;
        c.expect(&Tok::Colon, "':'")?;
        let value = ty(c)?;
        let constraints = if c.eat_word("where") {
            constraints(c)?
        } else {
            Vec::new()
        };
        let access = access(c)?;
        let scan = c.eat_word("scan");
        return Ok(Need::Kv(KvNeed {
            template,
            value,
            constraints,
            access,
            scan,
            span,
        }));
    }
    if c.eat_word("clock") {
        c.keyword("read")?;
        return Ok(Need::Clock { span });
    }
    if c.eat_word("effects") {
        c.expect(&Tok::Colon, "':'")?;
        let mut effects = vec![effect(c)?];
        while c.eat(&Tok::Comma) {
            effects.push(effect(c)?);
        }
        return Ok(Need::Effects { effects, span });
    }
    Err(c.unexpected("'kv', 'clock' or 'effects'"))
}

fn access(c: &mut Cursor<'_>) -> Result<Access, SyntaxError> {
    if let Some(read) = c.peek()
        && c.at_word("read")
    {
        c.advance();
        // "read-write" is one word in the grammar: no space around the '-'.
        if let (Some(minus), Some(write)) = (c.peek(), c.peek_at(1))
            && minus.tok == Tok::Minus
            && matches!(&write.tok, Tok::Word(w) if w == "write")
            && adjacent(read, minus)
            && adjacent(minus, write)
        {
            c.advance();
            c.advance();
            return Ok(Access::ReadWrite);
        }
        return Ok(Access::Read);
    }
    if c.eat_word("write") {
        return Ok(Access::Write);
    }
    Err(c.unexpected("'read', 'write' or 'read-write'"))
}

fn effect(c: &mut Cursor<'_>) -> Result<Effect, SyntaxError> {
    if c.eat_word("atomic") {
        return Ok(Effect::Atomic);
    }
    if c.eat_word("idempotent") {
        c.keyword("by")?;
        let key = path(c)?;
        c.keyword("for")?;
        let retention = duration(c)?;
        return Ok(Effect::Idempotent { key, retention });
    }
    Err(c.unexpected("'atomic' or 'idempotent'"))
}

fn duration(c: &mut Cursor<'_>) -> Result<Duration, SyntaxError> {
    if let Some(Tok::Word(w)) = c.peek_tok()
        && let Some(duration) = parse_duration(w)
    {
        c.advance();
        return Ok(duration);
    }
    Err(c.unexpected("a duration (such as 15min or 24h)"))
}

fn test(line: &Line) -> Result<Test, SyntaxError> {
    let mut c = Cursor::new(line);
    if c.eat_word("example") {
        let (name, _) = c.string("the name of the example")?;
        c.expect(&Tok::Colon, "':'")?;
        c.finish()?;
        return Ok(Test::Example(Box::new(example(line, name)?)));
    }
    if c.eat_word("generate") {
        c.expect(&Tok::Colon, "':'")?;
        c.finish()?;
        return Ok(Test::Generate(generate(line)?));
    }
    Err(c.unexpected("'example' or 'generate'"))
}

fn example(line: &Line, name: String) -> Result<Example, SyntaxError> {
    let mut rest = line.children.iter().peekable();
    let given = match rest.next_if(|l| starts_with(l, "given")) {
        Some(given) => whole(given, given_line)?,
        None => Given::default(),
    };
    let nothing = "the end of the example";
    let input = rest
        .next()
        .ok_or_else(|| SyntaxError::new(line.end, format!("expected 'input', found {nothing}")))?;
    let input = whole(input, |c| {
        c.keyword("input")?;
        value(c)
    })?;
    let expect = rest
        .next()
        .ok_or_else(|| SyntaxError::new(line.end, format!("expected 'expect', found {nothing}")))?;
    let expect = whole(expect, expect_line)?;
    if let Some(extra) = rest.next() {
        return Err(SyntaxError::new(
            extra.span,
            "unexpected line after 'expect'",
        ));
    }
    Ok(Example {
        name,
        given,
        input,
        expect,
        span: line.span,
    })
}

fn given_line(c: &mut Cursor<'_>) -> Result<Given, SyntaxError> {
    c.keyword("given")?;
    let mut given = Given::default();
    loop {
        let span = c.here();
        if c.eat_word("kv") {
            if given.kv.is_some() {
                return Err(SyntaxError::new(span, "'kv' is given twice"));
            }
            c.expect(&Tok::Eq, "'='")?;
            given.kv = Some(kv_map(c)?);
        } else if c.eat_word("clock") {
            if given.clock.is_some() {
                return Err(SyntaxError::new(span, "'clock' is given twice"));
            }
            c.expect(&Tok::Eq, "'='")?;
            given.clock = Some(c.string("a timestamp string")?.0);
        } else {
            return Err(c.unexpected("'kv' or 'clock'"));
        }
        if !c.eat(&Tok::Comma) {
            break;
        }
    }
    Ok(given)
}

fn expect_line(c: &mut Cursor<'_>) -> Result<Expect, SyntaxError> {
    c.keyword("expect")?;
    let outcome = if c.eat_word("ok") {
        Outcome::Ok(value(c)?)
    } else if c.eat_word("error") {
        let (name, _) = c.name("the name of an error")?;
        let value = if c.peek().is_some() && !c.at(&Tok::Comma) {
            Some(value(c)?)
        } else {
            None
        };
        Outcome::Error { name, value }
    } else {
        return Err(c.unexpected("'ok' or 'error'"));
    };
    let kv = if c.eat(&Tok::Comma) {
        c.keyword("kv")?;
        c.expect(&Tok::Eq, "'='")?;
        Some(kv_map(c)?)
    } else {
        None
    };
    Ok(Expect { outcome, kv })
}

fn kv_map(c: &mut Cursor<'_>) -> Result<KvMap, SyntaxError> {
    let span = c.expect(&Tok::LBrace, "'{'")?;
    let mut entries = Vec::new();
    if !c.eat(&Tok::RBrace) {
        loop {
            let key = match c.peek_tok() {
                Some(Tok::Word(key) | Tok::Str(key)) => key.clone(),
                _ => return Err(c.unexpected("a key")),
            };
            c.advance();
            c.expect(&Tok::Colon, "':'")?;
            entries.push((key, value(c)?));
            if !c.eat(&Tok::Comma) {
                break;
            }
        }
        c.expect(&Tok::RBrace, "',' or '}'")?;
    }
    Ok(KvMap { entries, span })
}

fn generate(line: &Line) -> Result<Generate, SyntaxError> {
    let mut generate = Generate {
        covers: Vec::new(),
        runs: None,
        min_hits: None,
        span: line.span,
    };
    for setting in &line.children {
        whole(setting, |c| {
            if c.eat_word("state") {
                c.keyword("kv")?;
                c.keyword("covers")?;
                generate.covers.push(path(c)?);
                return Ok(());
            }
            loop {
                let span = c.here();
                let (word, slot) = if c.eat_word("runs") {
                    ("runs", &mut generate.runs)
                } else if c.eat_word("min_hits") {
                    ("min_hits", &mut generate.min_hits)
                } else {
                    return Err(c.unexpected("'state', 'runs' or 'min_hits'"));
                };
                if slot.is_some() {
                    return Err(SyntaxError::new(span, format!("'{word}' is set twice")));
                }
                *slot = Some(c.count()?);
                if !c.eat(&Tok::Comma) {
                    return Ok(());
                }
            }
        })?;
    }
    Ok(generate)
}

fn value(c: &mut Cursor<'_>) -> Result<Value, SyntaxError> {
    let span = c.here();
    if let Some(digits) = negative_digits(c) {
        c.advance();
        c.advance();
        return Ok(Value {
            kind: ValueKind::Int(int(digits, true, span)?),
            span,
        });
    }
    let kind = match c.peek_tok() {
        Some(Tok::Word(w)) if is_digits(w) => {
            c.advance();
            ValueKind::Int(int(w, false, span)?)
        }
        Some(Tok::Word(w)) if w == "true" || w == "false" => {
            c.advance();
            ValueKind::Bool(w == "true")
        }
        Some(Tok::Str(s)) => {
            c.advance();
            ValueKind::Str(s.clone())
        }
        Some(Tok::LBracket) => {
            c.advance();
            let mut items = Vec::new();
            if !c.eat(&Tok::RBracket) {
                loop {
                    items.push(value(c)?);
                    if !c.eat(&Tok::Comma) {
                        break;
                    }
                }
                c.expect(&Tok::RBracket, "',' or ']'")?;
            }
            ValueKind::List(items)
        }
        Some(Tok::LBrace) => {
            c.advance();
            let mut fields = Vec::new();
            if !c.eat(&Tok::RBrace) {
                loop {
                    let (name, _) = c.ident("a field name")?;
                    c.expect(&Tok::Colon, "':'")?;
                    fields.push((name, value(c)?));
                    if !c.eat(&Tok::Comma) {
                        break;
                    }
                }
                c.expect(&Tok::RBrace, "',' or '}'")?;
            }
            ValueKind::Record(fields)
        }
        _ => return Err(c.unexpected("a value")),
    };
    Ok(Value { kind, span })
}

fn path(c: &mut Cursor<'_>) -> Result<Path, SyntaxError> {
    let (first, span) = c.variable("a name")?;
    let mut segments = vec![first];
    while c.eat(&Tok::Dot) {
        segments.push(c.ident("a field name")?.0);
    }
    Ok(Path { segments, span })
}

fn expr(c: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let span = c.here();
    let each = c.at_word("each");
    if each || c.at_word("any") {
        c.advance();
        let (var, _) = c.variable("a variable name")?;
        c.keyword("in")?;
        let list = path(c)?;
        c.expect(&Tok::Colon, "':'")?;
        let body = Box::new(expr(c)?);
        let kind = if each {
            ExprKind::Each { var, list, body }
        } else {
            ExprKind::Any { var, list, body }
        };
        return Ok(Expr { kind, span });
    }
    or_expr(c)
}

fn or_expr(c: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let mut left = and_expr(c)?;
    while c.eat_word("or") {
        let right = and_expr(c)?;
        left = Expr {
            span: left.span,
            kind: ExprKind::Or(Box::new(left), Box::new(right)),
        };
    }
    Ok(left)
}

fn and_expr(c: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let mut left = not_expr(c)?;
    while c.eat_word("and") {
        let right = not_expr(c)?;
        left = Expr {
            span: left.span,
            kind: ExprKind::And(Box::new(left), Box::new(right)),
        };
    }
    Ok(left)
}

fn not_expr(c: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let span = c.here();
    if c.eat_word("not") {
        let inner = cmp(c)?;
        return Ok(Expr {
            kind: ExprKind::Not(Box::new(inner)),
            span,
        });
    }
    cmp(c)
}

fn cmp(c: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let span = c.here();
    if c.eat_word("matches") {
        let (regex, _) = c.string("a regular expression string")?;
        return Ok(Expr {
            kind: ExprKind::Matches(regex),
            span,
        });
    }
    let first = sum(c)?;
    let negated =
        c.at_word("not") && matches!(c.peek_at(1).map(|t| &t.tok), Some(Tok::Word(w)) if w == "in");
    if negated {
        c.advance();
    }
    if c.eat_word("in") {
        let state = if c.eat_word("kv_before") {
            KvState::Before
        } else if c.eat_word("kv_after") {
            KvState::After
        } else {
            return Err(c.unexpected("'kv_before' or 'kv_after'"));
        };
        return Ok(Expr {
            kind: ExprKind::InKv {
                key: Box::new(first),
                state,
                negated,
            },
            span,
        });
    }
    let mut rest = Vec::new();
    while let Some(op) = c.peek_tok().and_then(cmp_op) {
        c.advance();
        rest.push((op, sum(c)?));
    }
    if rest.is_empty() {
        return Ok(first);
    }
    Ok(Expr {
        kind: ExprKind::Compare {
            first: Box::new(first),
            rest,
        },
        span,
    })
}

fn cmp_op(tok: &Tok) -> Option<CmpOp> {
    match tok {
        Tok::EqEq => Some(CmpOp::Eq),
        Tok::Ne => Some(CmpOp::Ne),
        Tok::Lt => Some(CmpOp::Lt),
        Tok::Le => Some(CmpOp::Le),
        Tok::Gt => Some(CmpOp::Gt),
        Tok::Ge => Some(CmpOp::Ge),
        _ => None,
    }
}

fn sum(c: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let mut left = term(c)?;
    loop {
        let add = c.at(&Tok::Plus);
        if !add && !c.at(&Tok::Minus) {
            return Ok(left);
        }
        c.advance();
        let right = Box::new(term(c)?);
        let span = left.span;
        let left_box = Box::new(left);
        left = Expr {
            kind: if add {
                ExprKind::Add(left_box, right)
            } else {
                ExprKind::Sub(left_box, right)
            },
            span,
        };
    }
}

fn term(c: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let span = c.here();
    if let Some(digits) = negative_digits(c) {
        c.advance();
        c.advance();
        return Ok(Expr {
            kind: ExprKind::Int(int(digits, true, span)?),
            span,
        });
    }
    let kind = match c.peek_tok() {
        Some(Tok::Str(s)) => {
            c.advance();
            ExprKind::Str(s.clone())
        }
        Some(Tok::LParen) => {
            c.advance();
            let inner = expr(c)?;
            c.expect(&Tok::RParen, "')'")?;
            return Ok(inner);
        }
        Some(Tok::Word(w)) => word_term(c, w, span)?,
        _ => return Err(c.unexpected("an expression")),
    };
    Ok(Expr { kind, span })
}

fn word_term(c: &mut Cursor<'_>, w: &str, span: Span) -> Result<ExprKind, SyntaxError> {
    if is_digits(w) {
        c.advance();
        return Ok(ExprKind::Int(int(w, false, span)?));
    }
    if let Some(duration) = parse_duration(w) {
        c.advance();
        return Ok(ExprKind::Duration(duration));
    }
    match w {
        "true" | "false" => {
            c.advance();
            Ok(ExprKind::Bool(w == "true"))
        }
        "kv_before" | "kv_after" => {
            let state = if w == "kv_before" {
                KvState::Before
            } else {
                KvState::After
            };
            c.advance();
            c.expect(&Tok::LBracket, "'['")?;
            let mut args = vec![expr(c)?];
            while c.eat(&Tok::Comma) {
                args.push(expr(c)?);
            }
            c.expect(&Tok::RBracket, "',' or ']'")?;
            Ok(ExprKind::KvIndex { state, args })
        }
        "len" | "unique" => {
            let len = w == "len";
            c.advance();
            c.expect(&Tok::LParen, "'('")?;
            let path = path(c)?;
            c.expect(&Tok::RParen, "')'")?;
            Ok(if len {
                ExprKind::Len(path)
            } else {
                ExprKind::Unique(path)
            })
        }
        _ if is_ident(w) && !RESERVED.contains(&w) => Ok(ExprKind::Path(path(c)?)),
        _ => Err(c.unexpected("an expression")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(path: &Path) -> String {
        path.segments.join(".")
    }

    fn state(state: KvState) -> &'static str {
        match state {
            KvState::Before => "kv_before",
            KvState::After => "kv_after",
        }
    }

    fn dur(d: &Duration) -> String {
        let unit = match d.unit {
            DurationUnit::Seconds => "s",
            DurationUnit::Minutes => "min",
            DurationUnit::Hours => "h",
            DurationUnit::Days => "d",
        };
        format!("{}{unit}", d.amount)
    }

    /// An expression as an S-expression, to compare the shape of the tree.
    fn sx(e: &Expr) -> String {
        let op = |op: &CmpOp| match op {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
        };
        match &e.kind {
            ExprKind::Each { var, list, body } => format!("(each {var} {} {})", p(list), sx(body)),
            ExprKind::Any { var, list, body } => format!("(any {var} {} {})", p(list), sx(body)),
            ExprKind::Or(a, b) => format!("(or {} {})", sx(a), sx(b)),
            ExprKind::And(a, b) => format!("(and {} {})", sx(a), sx(b)),
            ExprKind::Not(a) => format!("(not {})", sx(a)),
            ExprKind::Compare { first, rest } => {
                let mut s = format!("(cmp {}", sx(first));
                for (o, e) in rest {
                    s.push_str(&format!(" {} {}", op(o), sx(e)));
                }
                s + ")"
            }
            ExprKind::InKv {
                key,
                state: st,
                negated,
            } => {
                let word = if *negated { "not-in" } else { "in" };
                format!("({word} {} {})", sx(key), state(*st))
            }
            ExprKind::Matches(r) => format!("(matches {r:?})"),
            ExprKind::Add(a, b) => format!("(+ {} {})", sx(a), sx(b)),
            ExprKind::Sub(a, b) => format!("(- {} {})", sx(a), sx(b)),
            ExprKind::Int(n) => n.to_string(),
            ExprKind::Duration(d) => dur(d),
            ExprKind::Str(s) => format!("{s:?}"),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::KvIndex { state: st, args } => {
                let args: Vec<String> = args.iter().map(sx).collect();
                format!("({} {})", state(*st), args.join(" "))
            }
            ExprKind::Len(path) => format!("(len {})", p(path)),
            ExprKind::Unique(path) => format!("(unique {})", p(path)),
            ExprKind::Path(path) => p(path),
        }
    }

    fn ty_sx(t: &Type) -> String {
        match t {
            Type::Int => "int".into(),
            Type::String => "string".into(),
            Type::Bool => "bool".into(),
            Type::Timestamp => "timestamp".into(),
            Type::List(inner) => format!("list<{}>", ty_sx(inner)),
            Type::Record(fields) => {
                let fields: Vec<String> = fields.iter().map(field_sx).collect();
                format!("{{{}}}", fields.join(", "))
            }
        }
    }

    fn field_sx(f: &Field) -> String {
        if f.constraints.is_empty() {
            return format!("{}: {}", f.name, ty_sx(&f.ty));
        }
        let cs: Vec<String> = f.constraints.iter().map(sx).collect();
        format!("{}: {} where {}", f.name, ty_sx(&f.ty), cs.join(", "))
    }

    fn val(v: &Value) -> String {
        match &v.kind {
            ValueKind::Int(n) => n.to_string(),
            ValueKind::Str(s) => format!("{s:?}"),
            ValueKind::Bool(b) => b.to_string(),
            ValueKind::List(items) => {
                let items: Vec<String> = items.iter().map(val).collect();
                format!("[{}]", items.join(", "))
            }
            ValueKind::Record(fields) => {
                let fields: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", val(v)))
                    .collect();
                format!("{{{}}}", fields.join(", "))
            }
        }
    }

    fn kv(m: &KvMap) -> String {
        let entries: Vec<String> = m
            .entries
            .iter()
            .map(|(k, v)| format!("{k}: {}", val(v)))
            .collect();
        format!("{{{}}}", entries.join(", "))
    }

    fn parse_expr(source: &str) -> Result<Expr, SyntaxError> {
        let lines = lexer::lines(source)?;
        let line = lines.first().expect("one line");
        whole(line, expr)
    }

    fn e(source: &str) -> String {
        sx(&parse_expr(source).expect("the expression parses"))
    }

    fn e_err(source: &str) -> String {
        parse_expr(source)
            .expect_err("the expression is refused")
            .to_string()
    }

    #[test]
    fn precedence() {
        assert_eq!(e("a or b and not c"), "(or a (and b (not c)))");
        assert_eq!(e("(a or b) and c"), "(and (or a b) c)");
        assert_eq!(e("a and b and c"), "(and (and a b) c)");
        assert_eq!(e("a - b + c"), "(+ (- a b) c)");
        assert_eq!(e("not a == b"), "(not (cmp a == b))");
    }

    #[test]
    fn comparisons_chain() {
        assert_eq!(e("1 <= qty <= 99"), "(cmp 1 <= qty <= 99)");
        assert_eq!(e("a != b"), "(cmp a != b)");
        assert_eq!(e("a < b"), "(cmp a < b)");
        assert_eq!(e("a >= b > c"), "(cmp a >= b > c)");
    }

    #[test]
    fn kv_membership_and_index() {
        assert_eq!(e("item.sku in kv_before"), "(in item.sku kv_before)");
        assert_eq!(e("item.sku not in kv_after"), "(not-in item.sku kv_after)");
        assert_eq!(e("kv_before[a, b] == 1"), "(cmp (kv_before a b) == 1)");
        assert_eq!(
            e_err("x in kv"),
            "line 1, column 6: expected 'kv_before' or 'kv_after', found 'kv'"
        );
    }

    #[test]
    fn quantifiers_take_the_rest_of_the_line() {
        assert_eq!(
            e("each item in items: kv_after[item.sku] == kv_before[item.sku] - item.qty"),
            "(each item items (cmp (kv_after item.sku) == (- (kv_before item.sku) item.qty)))"
        );
        assert_eq!(
            e("any x in a.b: x > 1 or x < 0"),
            "(any x a.b (or (cmp x > 1) (cmp x < 0)))"
        );
    }

    #[test]
    fn literals_and_calls() {
        assert_eq!(
            e("reserved_until == clock + 15min"),
            "(cmp reserved_until == (+ clock 15min))"
        );
        assert_eq!(e("t + 30s - 2h + 1d"), "(+ (- (+ t 30s) 2h) 1d)");
        assert_eq!(e("matches \"[A-Z]{1,3}\""), "(matches \"[A-Z]{1,3}\")");
        assert_eq!(e("1 <= len(items) <= 50"), "(cmp 1 <= (len items) <= 50)");
        assert_eq!(e("unique(items.sku)"), "(unique items.sku)");
        assert_eq!(e("flag == true"), "(cmp flag == true)");
        assert_eq!(e("s == \"x\""), "(cmp s == \"x\")");
    }

    #[test]
    fn negative_integers_need_no_space() {
        assert_eq!(e("x >= -5"), "(cmp x >= -5)");
        assert_eq!(e("x - 5"), "(- x 5)");
        assert_eq!(
            e("-9223372036854775808 < x"),
            "(cmp -9223372036854775808 < x)"
        );
        assert_eq!(
            e_err("-x < 1"),
            "line 1, column 1: expected an expression, found '-'"
        );
        assert_eq!(
            e_err("x >= - 5"),
            "line 1, column 6: expected an expression, found '-'"
        );
        assert_eq!(
            e_err("x < 9223372036854775808"),
            "line 1, column 5: the integer is out of the 64-bit range"
        );
    }

    #[test]
    fn refused_expressions() {
        assert_eq!(
            e_err("a and"),
            "line 1, column 6: expected an expression, found the end of the line"
        );
        assert_eq!(
            e_err("each and in x: y"),
            "line 1, column 6: expected a variable name, found 'and'"
        );
        assert_eq!(
            e_err("each x items: y"),
            "line 1, column 8: expected 'in', found 'items'"
        );
        assert_eq!(
            e_err("a not b"),
            "line 1, column 3: expected the end of the line, found 'not'"
        );
        assert_eq!(
            e_err("1A == b"),
            "line 1, column 1: expected an expression, found '1A'"
        );
        assert_eq!(
            e_err("a b"),
            "line 1, column 3: expected the end of the line, found 'b'"
        );
        assert_eq!(
            e_err("15mins > a"),
            "line 1, column 1: expected an expression, found '15mins'"
        );
    }

    const SMALL: &str = "\
box Add v1
summary: adds two numbers
in:
  a: int where 0 <= a <= 10
  b: int where 0 <= b <= 10
out:
  ok: { sum: int }
  ensures:
    sum == a + b
  errors:
    TooBig { limit: 15 } when a + b > 15
needs:
  clock read
tests:
  example \"small\":
    input { a: 1, b: 2 }
    expect ok { sum: 3 }
";

    fn contract(source: &str) -> Contract {
        parse_contract(source).expect("the contract parses")
    }

    fn refused(source: &str) -> String {
        parse_contract(source)
            .expect_err("the contract is refused")
            .to_string()
    }

    #[test]
    fn a_small_contract() {
        let c = contract(SMALL);
        assert_eq!((c.name.as_str(), c.version), ("Add", 1));
        assert_eq!(c.summary, "adds two numbers");
        let inputs: Vec<String> = c.inputs.iter().map(field_sx).collect();
        assert_eq!(
            inputs,
            [
                "a: int where (cmp 0 <= a <= 10)",
                "b: int where (cmp 0 <= b <= 10)"
            ]
        );
        assert_eq!(ty_sx(&c.output.ok), "{sum: int}");
        let ensures: Vec<String> = c.output.ensures.iter().map(sx).collect();
        assert_eq!(ensures, ["(cmp sum == (+ a b))"]);
        let [error] = c.output.errors.as_slice() else {
            panic!("one error: {:?}", c.output.errors);
        };
        assert_eq!(error.name, "TooBig");
        let fields: Vec<(String, Option<String>)> = error
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.value.as_ref().map(sx)))
            .collect();
        assert_eq!(fields, [("limit".to_string(), Some("15".to_string()))]);
        assert_eq!(sx(&error.when), "(cmp (+ a b) > 15)");
        assert_eq!(
            c.needs,
            [Need::Clock {
                span: Span {
                    line: 13,
                    column: 3
                }
            }]
        );
        let [Test::Example(example)] = c.tests.as_slice() else {
            panic!("one example: {:?}", c.tests);
        };
        assert_eq!(example.name, "small");
        assert_eq!(example.given, Given::default());
        assert_eq!(val(&example.input), "{a: 1, b: 2}");
        assert!(
            matches!(&example.expect.outcome, Outcome::Ok(v) if val(v) == "{sum: 3}"),
            "{:?}",
            example.expect
        );
        assert_eq!(example.expect.kv, None);
    }

    #[test]
    fn needs_and_errors_may_be_left_out() {
        let source = SMALL
            .replace("  errors:\n    TooBig { limit: 15 } when a + b > 15\n", "")
            .replace("needs:\n  clock read\n", "");
        let c = contract(&source);
        assert!(c.output.errors.is_empty(), "{:?}", c.output.errors);
        assert!(c.needs.is_empty(), "{:?}", c.needs);
        let ensures: Vec<String> = c.output.ensures.iter().map(sx).collect();
        assert_eq!(ensures, ["(cmp sum == (+ a b))"]);
        assert_eq!(c.tests.len(), 1);

        let no_tests = source.replace(
            "tests:\n  example \"small\":\n    input { a: 1, b: 2 }\n    expect ok { sum: 3 }\n",
            "",
        );
        assert_eq!(
            refused(&no_tests),
            "line 10, column 1: expected 'needs:' or 'tests:', found the end of the input"
        );
    }

    #[test]
    fn positions() {
        let c = contract(SMALL);
        let spans: Vec<Span> = c.inputs.iter().map(|f| f.span).collect();
        assert_eq!(
            spans,
            [Span { line: 4, column: 3 }, Span { line: 5, column: 3 }]
        );
        let constraint = c
            .inputs
            .first()
            .and_then(|f| f.constraints.first())
            .expect("a constraint");
        assert_eq!(
            constraint.span,
            Span {
                line: 4,
                column: 16
            }
        );
        let error = c.output.errors.first().expect("an error");
        assert_eq!(
            error.span,
            Span {
                line: 11,
                column: 5
            }
        );
        assert_eq!(
            error.when.span,
            Span {
                line: 11,
                column: 31
            }
        );
        let example = match c.tests.first() {
            Some(Test::Example(example)) => example,
            other => panic!("an example: {other:?}"),
        };
        assert_eq!(
            example.span,
            Span {
                line: 15,
                column: 3
            }
        );
        assert_eq!(
            example.input.span,
            Span {
                line: 16,
                column: 11
            }
        );
    }

    #[test]
    fn record_fields_and_constraint_lists() {
        let source = SMALL.replace(
            "  b: int where 0 <= b <= 10\n",
            "  items: list<{ sku: string where matches \"A\", qty: int where 1 <= qty, qty <= 9 }>\n         where len(items) <= 3, unique(items.sku)\n",
        );
        let c = contract(&source);
        let inputs: Vec<String> = c.inputs.iter().map(field_sx).collect();
        assert_eq!(
            inputs,
            [
                "a: int where (cmp 0 <= a <= 10)",
                "items: list<{sku: string where (matches \"A\"), qty: int where (cmp 1 <= qty), (cmp qty <= 9)}> where (cmp (len items) <= 3), (unique items.sku)",
            ]
        );
    }

    #[test]
    fn needs() {
        let source = SMALL.replace(
            "  clock read\n",
            "  kv \"a/{x}\": int where 0 <= v <= 9 read-write scan\n  kv \"b/{x}/{y}\": bool read\n  kv \"c/{x}\": string write\n  clock read\n  effects: atomic, idempotent by order.id for 24h\n",
        );
        let c = contract(&source);
        let shown: Vec<String> = c
            .needs
            .iter()
            .map(|n| match n {
                Need::Kv(kv) => {
                    let cs: Vec<String> = kv.constraints.iter().map(sx).collect();
                    format!(
                        "kv {:?} {} [{}] {:?} scan={}",
                        kv.template,
                        ty_sx(&kv.value),
                        cs.join(", "),
                        kv.access,
                        kv.scan
                    )
                }
                Need::Clock { .. } => "clock".to_string(),
                Need::Effects { effects, .. } => {
                    let effects: Vec<String> = effects
                        .iter()
                        .map(|e| match e {
                            Effect::Atomic => "atomic".to_string(),
                            Effect::Idempotent { key, retention } => {
                                format!("idempotent {} {}", p(key), dur(retention))
                            }
                        })
                        .collect();
                    format!("effects {}", effects.join(", "))
                }
            })
            .collect();
        assert_eq!(
            shown,
            [
                "kv \"a/{x}\" int [(cmp 0 <= v <= 9)] ReadWrite scan=true",
                "kv \"b/{x}/{y}\" bool [] Read scan=false",
                "kv \"c/{x}\" string [] Write scan=false",
                "clock",
                "effects atomic, idempotent order.id 24h",
            ]
        );
    }

    #[test]
    fn read_write_is_one_word() {
        let source = SMALL.replace("  clock read\n", "  kv \"a/{x}\": int read - write\n");
        assert_eq!(
            refused(&source),
            "line 13, column 24: expected the end of the line, found '-'"
        );
    }

    #[test]
    fn examples_and_generate() {
        let source = SMALL.replace(
            "    input { a: 1, b: 2 }\n    expect ok { sum: 3 }\n",
            "    given  kv = { A1: 10, \"b c\": -2 }, clock = \"2026-11-01T10:00:00Z\"\n    input { a: 1, b: [true, false], c: {} }\n    expect ok { sum: 3 }, kv = { A1: 7 }\n  example \"failing\":\n    input {}\n    expect error TooBig\n  example \"failing with fields\":\n    given clock = \"2026-11-01T00:00:00Z\"\n    input {}\n    expect error TooBig { limit: 15 }, kv = {}\n  generate:\n    state kv covers items.sku\n    runs 1000, min_hits 50\n",
        );
        let c = contract(&source);
        let shown: Vec<String> = c
            .tests
            .iter()
            .map(|t| match t {
                Test::Example(x) => {
                    let outcome = match &x.expect.outcome {
                        Outcome::Ok(v) => format!("ok {}", val(v)),
                        Outcome::Error { name, value } => {
                            format!("error {name} {:?}", value.as_ref().map(val))
                        }
                    };
                    format!(
                        "{:?} given kv={:?} clock={:?} input={} {outcome} kv={:?}",
                        x.name,
                        x.given.kv.as_ref().map(kv),
                        x.given.clock,
                        val(&x.input),
                        x.expect.kv.as_ref().map(kv),
                    )
                }
                Test::Generate(g) => {
                    let covers: Vec<String> = g.covers.iter().map(p).collect();
                    format!("generate {covers:?} {:?} {:?}", g.runs, g.min_hits)
                }
            })
            .collect();
        assert_eq!(
            shown,
            [
                "\"small\" given kv=Some(\"{A1: 10, b c: -2}\") clock=Some(\"2026-11-01T10:00:00Z\") input={a: 1, b: [true, false], c: {}} ok {sum: 3} kv=Some(\"{A1: 7}\")",
                "\"failing\" given kv=None clock=None input={} error TooBig None kv=None",
                "\"failing with fields\" given kv=None clock=Some(\"2026-11-01T00:00:00Z\") input={} error TooBig Some(\"{limit: 15}\") kv=Some(\"{}\")",
                "generate [\"items.sku\"] Some(1000) Some(50)",
            ]
        );
    }

    #[test]
    fn refused_contracts() {
        let cases = [
            (
                "",
                "line 1, column 1: expected 'box', found the end of the input",
            ),
            (
                "box add v1\n",
                "line 1, column 5: expected the name of the box, found 'add'",
            ),
            (
                "box Add 1\n",
                "line 1, column 9: expected a version (v1, v2, ...), found '1'",
            ),
        ];
        for (source, want) in cases {
            assert_eq!(refused(source), want, "source: {source:?}");
        }
        let replaced = [
            (
                "  ensures:\n    sum == a + b\n",
                "",
                "line 8, column 3: expected 'ensures', found 'errors'",
            ),
            (
                "  errors:\n",
                "  error:\n",
                "line 10, column 3: expected 'errors:', found 'error'",
            ),
            (
                "tests:\n",
                "test:\n",
                "line 14, column 1: expected 'tests', found 'test'",
            ),
            (
                "  a: int where",
                "  a int where",
                "line 4, column 5: expected ':', found 'int'",
            ),
            (
                "  a: int where",
                "  a: float where",
                "line 4, column 6: expected a type, found 'float'",
            ),
            (
                "  clock read\n",
                "  http GET \"x\"\n",
                "line 13, column 3: expected 'kv', 'clock' or 'effects', found 'http'",
            ),
            (
                "    expect ok { sum: 3 }\n",
                "",
                "line 15, column 19: expected 'expect', found the end of the example",
            ),
            (
                "    input { a: 1, b: 2 }\n",
                "    input { a: yes }\n",
                "line 16, column 16: expected a value, found 'yes'",
            ),
            (
                "    input { a: 1, b: 2 }\n",
                "    given clock = \"x\", clock = \"y\"\n    input { a: 1, b: 2 }\n",
                "line 16, column 24: 'clock' is given twice",
            ),
            (
                "    expect ok { sum: 3 }\n",
                "    expect ok { sum: 3 }\n  generate:\n    runs 1, runs 2\n",
                "line 19, column 13: 'runs' is set twice",
            ),
            (
                "    expect ok { sum: 3 }\n",
                "    expect ok { sum: 3 }\nextra\n",
                "line 18, column 1: unexpected line after 'tests:'",
            ),
            (
                "    TooBig { limit: 15 } when a + b > 15\n",
                "    TooBig { limit: 15 }\n",
                "line 11, column 25: expected 'when', found the end of the line",
            ),
        ];
        for (from, to, want) in replaced {
            assert!(SMALL.contains(from), "SMALL has {from:?}");
            assert_eq!(
                refused(&SMALL.replacen(from, to, 1)),
                want,
                "{from:?} -> {to:?}"
            );
        }
    }

    /// The contract of docs/examples/reserve-stock.md, the first code block.
    fn reserve_stock() -> String {
        let doc = include_str!("../../../../docs/examples/reserve-stock.md");
        doc.split("```")
            .nth(1)
            .expect("a code block")
            .trim_start_matches('\n')
            .to_string()
    }

    #[test]
    fn the_reserve_stock_example_of_the_docs() {
        let c = contract(&reserve_stock());
        assert_eq!((c.name.as_str(), c.version), ("ReserveStock", 1));
        assert_eq!(c.summary, "注文に含まれる商品の在庫を引き当てる");
        let inputs: Vec<String> = c.inputs.iter().map(field_sx).collect();
        assert_eq!(
            inputs,
            [
                "order_id: string where (matches \"ORD-[0-9]{8}\")",
                "items: list<{sku: string where (matches \"[A-Z0-9]{1,16}\"), qty: int where (cmp 1 <= qty <= 99)}> where (cmp 1 <= (len items) <= 50), (unique items.sku)",
            ]
        );
        assert_eq!(ty_sx(&c.output.ok), "{reserved_until: timestamp}");
        let ensures: Vec<String> = c.output.ensures.iter().map(sx).collect();
        assert_eq!(
            ensures,
            [
                "(each item items (in item.sku kv_before))",
                "(each item items (cmp (kv_after item.sku) == (- (kv_before item.sku) item.qty)))",
                "(cmp reserved_until == (+ clock 15min))",
            ]
        );
        let errors: Vec<String> = c
            .output
            .errors
            .iter()
            .map(|e| {
                let fields: Vec<String> = e
                    .fields
                    .iter()
                    .map(|f| match &f.value {
                        Some(v) => format!("{}: {}", f.name, sx(v)),
                        None => f.name.clone(),
                    })
                    .collect();
                format!("{} {{{}}} when {}", e.name, fields.join(", "), sx(&e.when))
            })
            .collect();
        assert_eq!(
            errors,
            [
                "UnknownSku {sku} when (any item items (not-in item.sku kv_before))",
                "OutOfStock {sku} when (any item items (cmp item.qty > (kv_before item.sku)))",
            ]
        );
        let [
            Need::Kv(stock),
            Need::Clock { .. },
            Need::Effects { effects, .. },
        ] = c.needs.as_slice()
        else {
            panic!("kv, clock and effects: {:?}", c.needs);
        };
        assert_eq!(stock.template, "inventory/{sku}");
        assert_eq!(stock.value, Type::Int);
        let cs: Vec<String> = stock.constraints.iter().map(sx).collect();
        assert_eq!(cs, ["(cmp 0 <= v <= 100000)"]);
        assert_eq!((stock.access, stock.scan), (Access::ReadWrite, false));
        let [Effect::Atomic, Effect::Idempotent { key, retention }] = effects.as_slice() else {
            panic!("atomic and idempotent: {effects:?}");
        };
        assert_eq!(
            (p(key), dur(retention)),
            ("order_id".to_string(), "24h".to_string())
        );

        let [
            Test::Example(stocked),
            Test::Example(unknown),
            Test::Generate(generate),
        ] = c.tests.as_slice()
        else {
            panic!("two examples and generate: {:?}", c.tests);
        };
        assert_eq!(stocked.name, "在庫あり");
        assert_eq!(
            stocked.given.kv.as_ref().map(kv).as_deref(),
            Some("{A1: 10}")
        );
        assert_eq!(stocked.given.clock.as_deref(), Some("2026-11-01T10:00:00Z"));
        assert_eq!(
            val(&stocked.input),
            "{order_id: \"ORD-00000001\", items: [{sku: \"A1\", qty: 3}]}"
        );
        assert!(
            matches!(&stocked.expect.outcome, Outcome::Ok(v) if val(v) == "{reserved_until: \"2026-11-01T10:15:00Z\"}"),
            "{:?}",
            stocked.expect
        );
        assert_eq!(
            stocked.expect.kv.as_ref().map(kv).as_deref(),
            Some("{A1: 7}")
        );

        assert_eq!(unknown.name, "存在しない商品");
        assert_eq!(
            unknown.given.kv.as_ref().map(kv).as_deref(),
            Some("{A1: 10}")
        );
        assert_eq!(unknown.given.clock, None);
        assert_eq!(
            val(&unknown.input),
            "{order_id: \"ORD-00000002\", items: [{sku: \"B2\", qty: 1}]}"
        );
        assert!(
            matches!(&unknown.expect.outcome, Outcome::Error { name, value: Some(v) } if name == "UnknownSku" && val(v) == "{sku: \"B2\"}"),
            "{:?}",
            unknown.expect
        );
        assert_eq!(unknown.expect.kv, None);

        let covers: Vec<String> = generate.covers.iter().map(p).collect();
        assert_eq!(covers, ["items.sku"]);
        assert_eq!((generate.runs, generate.min_hits), (Some(1000), Some(50)));
    }
}
