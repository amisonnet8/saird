//! The syntax tree of contracts (docs/reference/syntax.md, "契約" and "式").
//!
//! It records what was written, with positions. Whether it makes sense (bounds,
//! names, types) is for the checker (docs/design/trust.md).

use super::error::Span;

/// A box contract (docs/reference/contract.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contract {
    pub name: String,
    pub version: u32,
    pub summary: String,
    pub inputs: Vec<Field>,
    pub output: Output,
    pub needs: Vec<Need>,
    pub tests: Vec<Test>,
}

/// A named value with its type and constraints: an input, or a field of a
/// record type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    /// The expressions after `where`; all of them must hold.
    pub constraints: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    String,
    Bool,
    Timestamp,
    List(Box<Type>),
    Record(Vec<Field>),
}

/// The `out` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub ok: Type,
    /// The `ok:` line.
    pub ok_span: Span,
    pub ensures: Vec<Expr>,
    /// In the order written: the first whose `when` holds is the one returned.
    pub errors: Vec<ErrorDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDecl {
    pub name: String,
    pub fields: Vec<ErrorField>,
    pub when: Expr,
    pub span: Span,
}

/// A field of an error. Without a value (`{ sku }`), it takes the field of the
/// same name from the element found by `any`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorField {
    pub name: String,
    pub value: Option<Expr>,
    pub span: Span,
}

/// A line of the `needs` section (docs/reference/permissions.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    Kv(KvNeed),
    Clock { span: Span },
    Effects { effects: Vec<Effect>, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvNeed {
    /// The key template, such as `inventory/{sku}`.
    pub template: String,
    /// The type of the values; constraints on them name the value `v`.
    pub value: Type,
    pub constraints: Vec<Expr>,
    pub access: Access,
    pub scan: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    ReadWrite,
}

/// docs/reference/effects.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Atomic,
    Idempotent { key: Path, retention: Duration },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    pub amount: u64,
    pub unit: DurationUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

/// A part of the `tests` section (docs/reference/generated-tests.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Test {
    Example(Box<Example>),
    Generate(Generate),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    pub name: String,
    pub given: Given,
    pub input: Value,
    pub expect: Expect,
    pub span: Span,
}

/// What an example starts from. What is left out has its default
/// (docs/reference/syntax.md, "例テスト"); that is applied when the example is
/// run, not here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Given {
    pub kv: Option<KvMap>,
    /// A timestamp, as written (RFC 3339 with `Z`).
    pub clock: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expect {
    pub outcome: Outcome,
    /// The whole kv range after the call.
    pub kv: Option<KvMap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ok(Value),
    Error { name: String, value: Option<Value> },
}

/// The kv range of an example. Keys are the values of the template's variable
/// (`A1` for the key `inventory/A1` of `inventory/{sku}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvMap {
    pub entries: Vec<(String, Value)>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generate {
    /// The paths of `state kv covers`.
    pub covers: Vec<Path>,
    pub runs: Option<u64>,
    pub min_hits: Option<u64>,
    pub span: Span,
}

/// A literal value of an example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub kind: ValueKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueKind {
    Int(i64),
    Str(String),
    Bool(bool),
    List(Vec<Value>),
    Record(Vec<(String, Value)>),
}

/// A name followed by field names: `items.sku`. Through a list, it stands for
/// the values of every element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub segments: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    /// `each var in list: body`
    Each {
        var: String,
        list: Path,
        body: Box<Expr>,
    },
    /// `any var in list: body`
    Any {
        var: String,
        list: Path,
        body: Box<Expr>,
    },
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    /// `first op e op e ...`: every comparison holds (`1 <= qty <= 99`).
    Compare {
        first: Box<Expr>,
        rest: Vec<(CmpOp, Expr)>,
    },
    /// `key in kv_before`, `key not in kv_after`
    InKv {
        key: Box<Expr>,
        state: KvState,
        negated: bool,
    },
    /// `matches "regex"`, on the field the constraint belongs to.
    Matches(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Int(i64),
    Duration(Duration),
    Str(String),
    Bool(bool),
    /// `kv_before[a, ...]`: one argument per variable of the key template.
    KvIndex {
        state: KvState,
        args: Vec<Expr>,
    },
    Len(Path),
    Unique(Path),
    Path(Path),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvState {
    Before,
    After,
}
