//! Parser for psh — to be rebuilt with combine 4.
//!
//! Scrapped and awaiting rewrite. See docs/syntax.md for the
//! normative grammar.

use anyhow::{bail, Result};

use crate::ast::*;

/// Parse the psh language. Public entry point.
pub struct PshParser;

impl PshParser {
    pub fn parse(_input: &str) -> Result<Program> {
        bail!("parser not yet implemented — awaiting combine rewrite")
    }
}

pub use PshParser as Parser;
