pub mod diagnostic;
pub mod parser;

pub use diagnostic::Diagnostic;
pub use parser::{parse_diagnostics, parse_primary};
