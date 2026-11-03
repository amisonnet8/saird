//! Types of contract values (docs/reference/syntax.md, "型と値") and the WIT
//! skeleton of a box (docs/reference/contract.md, "WIT との対応").
//!
//! The type of an expression is given here as it is when the expression is
//! well typed. Whether the operands fit (both sides of a comparison, bounds,
//! names) is for the checker.

mod timestamp;
mod wit;

pub use timestamp::Timestamp;
pub use wit::contract_wit;

use std::fmt;

use crate::syntax::Span;
use crate::syntax::ast::{Expr, ExprKind, Field, KvNeed, Path, Type};

/// A type of a value or an expression. A record is the one declared in the
/// contract, so records declared in different places are different types.
#[derive(Debug, Clone)]
pub enum Ty<'a> {
    Int,
    String,
    Bool,
    Timestamp,
    /// Only of an expression (`15min`); no value has it.
    Duration,
    List(Box<Ty<'a>>),
    Record(&'a [Field]),
}

impl<'a> Ty<'a> {
    pub fn of(ty: &'a Type) -> Self {
        match ty {
            Type::Int => Ty::Int,
            Type::String => Ty::String,
            Type::Bool => Ty::Bool,
            Type::Timestamp => Ty::Timestamp,
            Type::List(element) => Ty::List(Box::new(Ty::of(element))),
            Type::Record(fields) => Ty::Record(fields),
        }
    }
}

impl PartialEq for Ty<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Ty::Int, Ty::Int)
            | (Ty::String, Ty::String)
            | (Ty::Bool, Ty::Bool)
            | (Ty::Timestamp, Ty::Timestamp)
            | (Ty::Duration, Ty::Duration) => true,
            (Ty::List(a), Ty::List(b)) => a == b,
            (Ty::Record(a), Ty::Record(b)) => std::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl Eq for Ty<'_> {}

impl fmt::Display for Ty<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Int => f.write_str("int"),
            Ty::String => f.write_str("string"),
            Ty::Bool => f.write_str("bool"),
            Ty::Timestamp => f.write_str("timestamp"),
            Ty::Duration => f.write_str("a duration"),
            Ty::List(element) => write!(f, "list<{element}>"),
            Ty::Record(fields) => {
                let names: Vec<&str> = fields.iter().map(|field| field.name.as_str()).collect();
                write!(f, "{{ {} }}", names.join(", "))
            }
        }
    }
}

/// A contract whose types cannot be given, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub span: Span,
    pub message: String,
}

impl TypeError {
    pub(crate) fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span, self.message)
    }
}

impl std::error::Error for TypeError {}

/// The names an expression can use: variables of `each` / `any`, the inputs,
/// and `clock`, in that order; and the kv range that `kv_before[...]` reads.
pub(crate) struct Scope<'a> {
    inputs: &'a [Field],
    kv: Vec<&'a KvNeed>,
    vars: Vec<(&'a str, Ty<'a>)>,
}

impl<'a> Scope<'a> {
    pub(crate) fn new(inputs: &'a [Field], kv: Vec<&'a KvNeed>) -> Self {
        Self {
            inputs,
            kv,
            vars: Vec::new(),
        }
    }

    pub(crate) fn bind(&mut self, name: &'a str, ty: Ty<'a>) {
        self.vars.push((name, ty));
    }

    pub(crate) fn type_of(&self, expr: &Expr) -> Result<Ty<'a>, TypeError> {
        match &expr.kind {
            ExprKind::Int(_) => Ok(Ty::Int),
            ExprKind::Str(_) => Ok(Ty::String),
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Duration(_) => Ok(Ty::Duration),
            ExprKind::Each { .. }
            | ExprKind::Any { .. }
            | ExprKind::Or(..)
            | ExprKind::And(..)
            | ExprKind::Not(_)
            | ExprKind::Compare { .. }
            | ExprKind::InKv { .. }
            | ExprKind::Matches(_)
            | ExprKind::Unique(_) => Ok(Ty::Bool),
            ExprKind::Len(_) => Ok(Ty::Int),
            ExprKind::Add(a, b) => self.arithmetic(expr.span, '+', a, b),
            ExprKind::Sub(a, b) => self.arithmetic(expr.span, '-', a, b),
            ExprKind::KvIndex { .. } => match self.kv.as_slice() {
                [kv] => Ok(Ty::of(&kv.value)),
                [] => Err(TypeError::new(expr.span, "no kv range in 'needs:'")),
                _ => Err(TypeError::new(
                    expr.span,
                    "more than one kv range in 'needs:'",
                )),
            },
            ExprKind::Path(path) => self.path(path),
        }
    }

    /// `int + int`, and `timestamp` plus or minus a duration
    /// (docs/reference/syntax.md, "型と値").
    fn arithmetic(&self, span: Span, op: char, a: &Expr, b: &Expr) -> Result<Ty<'a>, TypeError> {
        match (self.type_of(a)?, self.type_of(b)?) {
            (Ty::Int, Ty::Int) => Ok(Ty::Int),
            (Ty::Timestamp, Ty::Duration) => Ok(Ty::Timestamp),
            (left, right) => Err(TypeError::new(
                span,
                format!(
                    "'{op}' needs int {op} int or timestamp {op} a duration, found {left} {op} {right}"
                ),
            )),
        }
    }

    /// Through a list, a path stands for the values of every element
    /// (`items.sku` is a `list<string>`).
    pub(crate) fn path(&self, path: &Path) -> Result<Ty<'a>, TypeError> {
        let (first, rest) = path
            .segments
            .split_first()
            .expect("the parser gives a path at least one name");
        let mut ty = self
            .lookup(first)
            .ok_or_else(|| TypeError::new(path.span, format!("unknown name '{first}'")))?;
        let mut through_list = false;
        for segment in rest {
            let record = match ty {
                Ty::List(element) if !through_list => {
                    through_list = true;
                    *element
                }
                Ty::List(_) => {
                    return Err(TypeError::new(
                        path.span,
                        format!("'{}' goes through more than one list", dotted(path)),
                    ));
                }
                other => other,
            };
            let Ty::Record(fields) = record else {
                return Err(TypeError::new(
                    path.span,
                    format!("'{segment}' is not a field of {record}"),
                ));
            };
            let field = fields
                .iter()
                .find(|field| field.name == *segment)
                .ok_or_else(|| {
                    TypeError::new(path.span, format!("'{segment}' is not a field of {record}"))
                })?;
            ty = Ty::of(&field.ty);
        }
        Ok(if through_list {
            Ty::List(Box::new(ty))
        } else {
            ty
        })
    }

    /// The type of an element of the list a path stands for (`each x in p`).
    pub(crate) fn element(&self, path: &Path) -> Result<Ty<'a>, TypeError> {
        match self.path(path)? {
            Ty::List(element) => Ok(*element),
            other => Err(TypeError::new(
                path.span,
                format!("'{}' is {other}, not a list", dotted(path)),
            )),
        }
    }

    fn lookup(&self, name: &str) -> Option<Ty<'a>> {
        if let Some((_, ty)) = self.vars.iter().rev().find(|(var, _)| *var == name) {
            return Some(ty.clone());
        }
        if let Some(field) = self.inputs.iter().find(|field| field.name == name) {
            return Some(Ty::of(&field.ty));
        }
        (name == "clock").then_some(Ty::Timestamp)
    }
}

fn dotted(path: &Path) -> String {
    path.segments.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse_contract;

    #[test]
    fn a_record_is_equal_only_to_itself() {
        let source = "box T v1\nsummary: t\nin:\n  a: { x: bool }\n  b: { x: bool }\nout:\n  ok: bool\n  ensures:\n    true\ntests:\n  generate:\n    runs 10\n";
        let contract = parse_contract(source).expect("the source parses");
        let (a, b) = (&contract.inputs[0].ty, &contract.inputs[1].ty);
        assert_eq!(Ty::of(a), Ty::of(a));
        assert_ne!(Ty::of(a), Ty::of(b));
    }

    #[test]
    fn other_types_are_equal_by_their_shape() {
        let list = |ty| Ty::List(Box::new(ty));
        let all = [
            Ty::Int,
            Ty::String,
            Ty::Bool,
            Ty::Timestamp,
            Ty::Duration,
            list(Ty::Int),
            list(Ty::String),
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                assert_eq!(a == b, i == j, "{a} == {b}");
            }
        }
        assert_eq!(list(Ty::Int), list(Ty::Int));
    }

    #[test]
    fn a_type_error_shows_where_it_is() {
        let error = TypeError::new(Span { line: 4, column: 3 }, "bad");
        assert_eq!(error.to_string(), "line 4, column 3: bad");
    }
}
