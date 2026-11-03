//! Reading contracts into a syntax tree (docs/reference/syntax.md).
//!
//! Only the grammar is checked here. The checker decides whether a contract
//! that parses makes sense (docs/design/trust.md, "保証の分担").

pub mod ast;
mod error;
mod lexer;
mod parser;

pub use error::{Span, SyntaxError};
pub use parser::parse_contract;
