//! The WIT skeleton of a box (docs/reference/contract.md, "WIT との対応").

use super::{Scope, Ty, TypeError};
use crate::syntax::Span;
use crate::syntax::ast::{Contract, ErrorDecl, ExprKind, Field, KvNeed, Need, Path, Type};

/// Words of WIT that a name cannot be without `%` (wit-parser, `ast/lex.rs`).
const KEYWORDS: [&str; 43] = [
    "as",
    "async",
    "bool",
    "borrow",
    "char",
    "constructor",
    "enum",
    "error-context",
    "export",
    "f32",
    "f64",
    "flags",
    "from",
    "func",
    "future",
    "import",
    "include",
    "interface",
    "list",
    "map",
    "option",
    "own",
    "package",
    "record",
    "resource",
    "result",
    "s16",
    "s32",
    "s64",
    "s8",
    "static",
    "stream",
    "string",
    "tuple",
    "type",
    "u16",
    "u32",
    "u64",
    "u8",
    "use",
    "variant",
    "with",
    "world",
];

const TIMESTAMP: &str = "  record timestamp {\n    seconds: s64,\n    nanoseconds: u32,\n  }\n";

/// The WIT of a box: its package, its world, and the function `call` with the
/// types of its inputs, its `ok` and its errors.
pub fn contract_wit(contract: &Contract) -> Result<String, TypeError> {
    let kv: Vec<_> = contract
        .needs
        .iter()
        .filter_map(|need| match need {
            Need::Kv(kv) => Some(kv),
            _ => None,
        })
        .collect();

    let mut records = Records::default();
    for input in &contract.inputs {
        let name = format!("in-{}", field_name(&input.name, input.span)?);
        records.declare(&input.ty, name, input.span)?;
    }
    let output = &contract.output;
    records.declare(&output.ok, "out-ok".to_owned(), output.ok_span)?;
    for need in &kv {
        records.declare(&need.value, "kv".to_owned(), need.span)?;
    }

    let mut params = Vec::new();
    for input in &contract.inputs {
        let name = field_name(&input.name, input.span)?;
        params.push((
            name,
            records.render(&Ty::of(&input.ty), input.span)?,
            input.span,
        ));
    }
    unique(&params, "input")?;
    let ok = records.render(&Ty::of(&output.ok), output.ok_span)?;

    let mut cases = Vec::new();
    let mut error_records = Vec::new();
    for decl in &output.errors {
        let case = pascal_name(&decl.name);
        if decl.fields.is_empty() {
            cases.push((escape(&case), String::new(), decl.span));
            continue;
        }
        let record = format!("error-{case}");
        let fields = error_fields(contract, &kv, decl, &mut records)?;
        unique(&fields, "field")?;
        cases.push((escape(&case), format!("({record})"), decl.span));
        error_records.push((record, fields, decl.span));
    }
    unique(&cases, "error")?;

    // Nested records are used once the records holding them are written.
    let mut bodies: Vec<Option<Vec<(String, String, Span)>>> = vec![None; records.list.len()];
    while let Some(index) =
        (0..records.list.len()).find(|&i| records.used[i] && bodies[i].is_none())
    {
        let fields = records.list[index].fields;
        let mut body = Vec::new();
        for field in fields {
            let name = field_name(&field.name, field.span)?;
            body.push((
                name,
                records.render(&Ty::of(&field.ty), field.span)?,
                field.span,
            ));
        }
        unique(&body, "field")?;
        bodies[index] = Some(body);
    }

    let mut names = Vec::new();
    for (record, body) in records.list.iter().zip(&bodies) {
        if body.is_some() {
            names.push((record.name.clone(), String::new(), record.span));
        }
    }
    for (name, _, span) in &error_records {
        names.push((name.clone(), String::new(), *span));
    }
    unique(&names, "record")?;

    let world = pascal_name(&contract.name);
    let mut wit = format!(
        "package saird-box:{}@{}.0.0;\n\nworld {} {{\n",
        escape(&world),
        contract.version,
        escape(&world)
    );
    if records.timestamp {
        wit.push_str(TIMESTAMP);
    }
    for (record, body) in records.list.iter().zip(&bodies) {
        if let Some(body) = body {
            wit.push_str(&record_wit(&record.name, body));
        }
    }
    for (name, fields, _) in &error_records {
        wit.push_str(&record_wit(name, fields));
    }
    let result = if cases.is_empty() {
        ok
    } else {
        wit.push_str("  variant error {\n");
        for (case, payload, _) in &cases {
            wit.push_str(&format!("    {case}{payload},\n"));
        }
        wit.push_str("  }\n");
        format!("result<{ok}, error>")
    };
    let params: Vec<String> = params
        .iter()
        .map(|(name, ty, _)| format!("{}: {ty}", escape(name)))
        .collect();
    wit.push_str(&format!(
        "  export call: func({}) -> {result};\n}}\n",
        params.join(", ")
    ));
    Ok(wit)
}

/// The fields of an error, with the types of their values. `{ sku }` takes
/// the field of the element that the `any` of `when` found.
fn error_fields<'a>(
    contract: &'a Contract,
    kv: &[&'a KvNeed],
    decl: &'a ErrorDecl,
    records: &mut Records<'a>,
) -> Result<Vec<(String, String, Span)>, TypeError> {
    let mut scope = Scope::new(&contract.inputs, kv.to_vec());
    let found = match &decl.when.kind {
        ExprKind::Any { var, list, .. } => {
            scope.bind(var, scope.element(list)?);
            Some(var)
        }
        _ => None,
    };
    let mut fields = Vec::new();
    for field in &decl.fields {
        let ty = match (&field.value, found) {
            (Some(value), _) => scope.type_of(value)?,
            (None, Some(var)) => scope.path(&Path {
                segments: vec![var.clone(), field.name.clone()],
                span: field.span,
            })?,
            (None, None) => {
                return Err(TypeError::new(
                    field.span,
                    format!(
                        "'{}' has no value, and the 'when' of {} does not start with 'any'",
                        field.name, decl.name
                    ),
                ));
            }
        };
        let name = field_name(&field.name, field.span)?;
        fields.push((name, records.render(&ty, field.span)?, field.span));
    }
    Ok(fields)
}

struct Record<'a> {
    name: String,
    fields: &'a [Field],
    span: Span,
}

/// The records declared in the contract, with the names they get in WIT, and
/// which of them the WIT uses.
#[derive(Default)]
struct Records<'a> {
    list: Vec<Record<'a>>,
    used: Vec<bool>,
    timestamp: bool,
}

impl<'a> Records<'a> {
    /// Names the records in `ty` and in its fields. A list adds nothing to the
    /// name of its element.
    fn declare(&mut self, ty: &'a Type, name: String, span: Span) -> Result<(), TypeError> {
        match ty {
            Type::List(element) => self.declare(element, name, span),
            Type::Record(fields) => {
                for field in fields {
                    let inner = format!("{name}-{}", field_name(&field.name, field.span)?);
                    self.declare(&field.ty, inner, field.span)?;
                }
                // The record before the ones inside it.
                let index = self.list.len() - count_records(fields);
                self.list.insert(index, Record { name, fields, span });
                self.used.insert(index, false);
                Ok(())
            }
            Type::Int | Type::String | Type::Bool | Type::Timestamp => Ok(()),
        }
    }

    fn render(&mut self, ty: &Ty<'a>, span: Span) -> Result<String, TypeError> {
        Ok(match ty {
            Ty::Int => "s64".to_owned(),
            Ty::String => "string".to_owned(),
            Ty::Bool => "bool".to_owned(),
            Ty::Timestamp => {
                self.timestamp = true;
                "timestamp".to_owned()
            }
            Ty::Duration => {
                return Err(TypeError::new(span, "a duration cannot be a value"));
            }
            Ty::List(element) => format!("list<{}>", self.render(element, span)?),
            Ty::Record(fields) => {
                let index = self
                    .list
                    .iter()
                    .position(|record| std::ptr::eq(record.fields, *fields))
                    .expect("every record of a type comes from a declared one");
                self.used[index] = true;
                escape(&self.list[index].name)
            }
        })
    }
}

/// How many records `fields` declare, inside them at any depth.
fn count_records(fields: &[Field]) -> usize {
    fn of(ty: &Type) -> usize {
        match ty {
            Type::List(element) => of(element),
            Type::Record(fields) => 1 + count_records(fields),
            Type::Int | Type::String | Type::Bool | Type::Timestamp => 0,
        }
    }
    fields.iter().map(|field| of(&field.ty)).sum()
}

fn record_wit(name: &str, fields: &[(String, String, Span)]) -> String {
    let mut wit = format!("  record {} {{\n", escape(name));
    for (field, ty, _) in fields {
        wit.push_str(&format!("    {}: {ty},\n", escape(field)));
    }
    wit.push_str("  }\n");
    wit
}

/// `order_id` is `order-id`. Without `%`: see [`escape`].
fn field_name(name: &str, span: Span) -> Result<String, TypeError> {
    let words: Vec<&str> = name.split('_').collect();
    let fits = words
        .iter()
        .all(|word| word.chars().next().is_some_and(|c| c.is_ascii_lowercase()));
    if !fits {
        return Err(TypeError::new(
            span,
            format!("'{name}' has no name in WIT: each word between '_' must start with a letter"),
        ));
    }
    Ok(words.join("-"))
}

/// `ReserveStock` is `reserve-stock`. Without `%`: see [`escape`].
fn pascal_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            if !out.is_empty() {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A name as WIT writes it: `%` before a keyword.
fn escape(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        format!("%{name}")
    } else {
        name.to_owned()
    }
}

/// Refuses a name given twice; the error is at the second.
fn unique(items: &[(String, String, Span)], what: &str) -> Result<(), TypeError> {
    for (i, (name, _, span)) in items.iter().enumerate() {
        if items[..i].iter().any(|(earlier, _, _)| earlier == name) {
            return Err(TypeError::new(
                *span,
                format!("'{name}' is the WIT name of two {what}s"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse_contract;

    fn wit(source: &str) -> Result<String, TypeError> {
        contract_wit(&parse_contract(source).expect("the source parses"))
    }

    /// A contract with one input line (line 4) and the lines of `errors:`
    /// (from line 9).
    fn source(inputs: &str, ok: &str, errors: &str) -> String {
        format!(
            "box T v1\nsummary: t\nin:\n  {inputs}\nout:\n  ok: {ok}\n  ensures:\n    true\n{errors}tests:\n  generate:\n    runs 10\n"
        )
    }

    fn error_at(line: usize, column: usize, message: &str) -> Result<String, TypeError> {
        Err(TypeError::new(Span { line, column }, message))
    }

    /// The code blocks of docs/examples/reserve-stock.md: the contract, then
    /// its WIT.
    fn reserve_stock() -> (String, String) {
        let doc = include_str!("../../../../docs/examples/reserve-stock.md");
        let blocks: Vec<&str> = doc.split("```").collect();
        let contract = blocks[1].trim_start_matches('\n').to_owned();
        let wit = blocks[3]
            .strip_prefix("wit\n")
            .expect("a wit block")
            .to_owned();
        (contract, wit)
    }

    #[test]
    fn the_reserve_stock_example_of_the_docs() {
        let (contract, expected) = reserve_stock();
        assert_eq!(wit(&contract), Ok(expected));
    }

    #[test]
    fn a_box_without_errors_returns_its_ok() {
        let source = "box AddTwo v2\nsummary: adds\nin:\n  a: int where 0 <= a <= 9\n  b: int where 0 <= b <= 9\nout:\n  ok: int\n  ensures:\n    true\ntests:\n  generate:\n    runs 10\n";
        assert_eq!(
            wit(source),
            Ok("package saird-box:add-two@2.0.0;\n\nworld add-two {\n  export call: func(a: s64, b: s64) -> s64;\n}\n".to_owned())
        );
    }

    #[test]
    fn names_nested_records_by_their_paths() {
        let source = source(
            "type: { list_of: list<{ x: bool }> }\n  when_at: timestamp",
            "{ items: list<list<string>> }",
            "",
        );
        let expected = "package saird-box:t@1.0.0;

world t {
  record timestamp {
    seconds: s64,
    nanoseconds: u32,
  }
  record in-type {
    list-of: list<in-type-list-of>,
  }
  record in-type-list-of {
    x: bool,
  }
  record out-ok {
    items: list<list<string>>,
  }
  export call: func(%type: in-type, when-at: timestamp) -> out-ok;
}
";
        assert_eq!(wit(&source), Ok(expected.to_owned()));
    }

    #[test]
    fn gives_error_fields_the_types_of_their_values() {
        let source = "box Pick v3
summary: picks
in:
  items: list<{ sku: string where len(sku) <= 8, qty: int where 0 <= qty <= 9 }> where len(items) <= 5
out:
  ok: bool
  ensures:
    true
  errors:
    Empty when len(items) == 0
    TooMany { count: len(items), at: clock + 1h } when len(items) > 4
    Bad { sku, qty } when any item in items: item.qty == 0
    Low { left: kv_before[x.sku], first: x } when any x in items: x.sku not in kv_before
needs:
  kv \"stock/{sku}\": { n: int where 0 <= n <= 9 } read
  clock read
tests:
  generate:
    runs 10
";
        let expected = "package saird-box:pick@3.0.0;

world pick {
  record timestamp {
    seconds: s64,
    nanoseconds: u32,
  }
  record in-items {
    sku: string,
    qty: s64,
  }
  record kv {
    n: s64,
  }
  record error-too-many {
    count: s64,
    at: timestamp,
  }
  record error-bad {
    sku: string,
    qty: s64,
  }
  record error-low {
    left: kv,
    first: in-items,
  }
  variant error {
    empty,
    too-many(error-too-many),
    bad(error-bad),
    low(error-low),
  }
  export call: func(items: list<in-items>) -> result<bool, error>;
}
";
        assert_eq!(wit(source), Ok(expected.to_owned()));
    }

    #[test]
    fn puts_a_record_before_the_records_inside_it() {
        let source = source("a: { b: { c: { d: bool } } }\n  e: { f: bool }", "bool", "");
        let expected = "package saird-box:t@1.0.0;

world t {
  record in-a {
    b: in-a-b,
  }
  record in-a-b {
    c: in-a-b-c,
  }
  record in-a-b-c {
    d: bool,
  }
  record in-e {
    f: bool,
  }
  export call: func(a: in-a, e: in-e) -> bool;
}
";
        assert_eq!(wit(&source), Ok(expected.to_owned()));
    }

    #[test]
    fn a_path_through_a_list_is_a_list() {
        let source = source(
            "items: list<{ sku: string }>",
            "bool",
            "  errors:\n    E { skus: items.sku } when true\n",
        );
        assert!(
            wit(&source)
                .expect("the WIT")
                .contains("  record error-e {\n    skus: list<string>,\n  }\n")
        );
    }

    #[test]
    fn refuses_kv_without_a_kv_range() {
        let source = source(
            "k: string",
            "bool",
            "  errors:\n    E { v: kv_before[k] } when true\n",
        );
        assert_eq!(wit(&source), error_at(10, 12, "no kv range in 'needs:'"));
    }

    #[test]
    fn refuses_names_that_have_no_wit_name() {
        for name in ["_x", "a__b", "a_", "item_2"] {
            assert_eq!(
                wit(&source(&format!("{name}: bool"), "bool", "")),
                error_at(
                    4,
                    3,
                    &format!(
                        "'{name}' has no name in WIT: each word between '_' must start with a letter"
                    )
                ),
                "{name}"
            );
        }
        assert_eq!(
            wit(&source("r: { ok_1: bool }", "bool", "")),
            error_at(
                4,
                8,
                "'ok_1' has no name in WIT: each word between '_' must start with a letter"
            )
        );
    }

    #[test]
    fn refuses_records_with_the_same_wit_name() {
        let source = source("a_b: { x: bool }\n  a: { b: { y: bool } }", "bool", "");
        assert_eq!(
            wit(&source),
            error_at(5, 8, "'in-a-b' is the WIT name of two records")
        );
    }

    #[test]
    fn refuses_error_fields_without_a_wit_type() {
        let errors = |decl: &str| format!("  errors:\n    {decl}\n");
        let input = "l: list<{ m: list<{ t: bool }> }>";
        assert_eq!(
            wit(&source(input, "bool", &errors("E { d: 15min } when true"))),
            error_at(10, 9, "a duration cannot be a value")
        );
        assert_eq!(
            wit(&source(input, "bool", &errors("E { sku } when true"))),
            error_at(
                10,
                9,
                "'sku' has no value, and the 'when' of E does not start with 'any'"
            )
        );
        assert_eq!(
            wit(&source(input, "bool", &errors("E { v: l.m.t } when true"))),
            error_at(10, 12, "'l.m.t' goes through more than one list")
        );
        assert_eq!(
            wit(&source(
                input,
                "bool",
                &errors("E { v: 1 + true } when true")
            )),
            error_at(
                10,
                12,
                "'+' needs int + int or timestamp + a duration, found int + bool"
            )
        );
    }
}
