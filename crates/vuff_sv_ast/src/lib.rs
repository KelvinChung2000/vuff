//! Thin ergonomic wrapper over `sv-parser`.
//!
//! sv-parser is a two-step pipeline: preprocess → parse. Token `Locate`
//! offsets are into the **preprocessed** text, not the caller-provided
//! source. This module exposes a `Parsed` struct carrying both the tree
//! and the preprocessed text so the formatter can extract inter-token
//! trivia consistently with the offsets.
//!
//! For sources without `` `include`` / `` `define`` / conditional compile
//! directives, the preprocessed text is (nearly) identical to the input.
//! For sources that do use them, we format the *post-preprocess* view,
//! which is what sv-parser considers the source-of-truth.

pub use sv_parser::{
    ConditionalExpression, DirectiveDetail, DirectiveKind, DirectiveSpan, FunctionStatementOrNull,
    IfdefBranch, IfdefBranchKind, IfdefChain, IncludeDirective, Locate, MacroDef, MacroDefArg,
    MacroUsage, NodeEvent, PpRange, PreprocessedText, RefNode, Statement, StatementItem,
    StatementOrNull, SyntaxTree,
};

use std::path::Path;

use sv_parser::{parse_sv_pp, preprocess_str, Defines};

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("sv-parser preprocess: {0}")]
    Preprocess(String),
    #[error("sv-parser parse: {0}")]
    Parse(String),
}

/// The result of parsing — tree plus the preprocessed source used as the
/// basis for token offsets, and the untouched caller-supplied source
/// kept alongside for directive re-splicing.
pub struct Parsed {
    pub tree: SyntaxTree,
    pub text: String,
    pub original: String,
    pub original_path: std::path::PathBuf,
}

impl Parsed {
    /// Map a byte position in [`Self::text`] (the preprocessed source) to
    /// the corresponding byte in [`Self::original`]. `None` if the pp
    /// byte was synthesized or came from an `` `include `` of a different
    /// file.
    #[must_use]
    pub fn origin_in_original(&self, pp_pos: usize) -> Option<usize> {
        let loc = Locate {
            offset: pp_pos,
            line: 0,
            len: 1,
        };
        let (path, orig_pos) = self.tree.get_origin(&loc)?;
        if path.as_path() == self.original_path {
            Some(orig_pos)
        } else {
            None
        }
    }
}

pub fn parse(source: &str, path: &Path) -> Result<Parsed, ParseError> {
    let includes: Vec<&Path> = Vec::new();
    let defines: Defines = std::collections::HashMap::new();
    let (pp, defines_out) = preprocess_str(source, path, &defines, &includes, false, false, 0, 0)
        .map_err(|e| ParseError::Preprocess(format!("{e:?}")))?;
    let text = pp.text().to_owned();
    let (tree, _) =
        parse_sv_pp(pp, defines_out, false).map_err(|e| ParseError::Parse(format!("{e:?}")))?;
    Ok(Parsed {
        tree,
        text,
        original: source.to_owned(),
        original_path: path.to_owned(),
    })
}

/// A single token seen by the parser, in source order.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    /// Byte offset into the **preprocessed** source.
    pub offset: usize,
    /// Byte length of the token text.
    pub len: usize,
    /// Token text (== preprocessed[offset..offset+len]).
    pub text: &'a str,
}

impl Token<'_> {
    #[must_use]
    pub const fn end(&self) -> usize {
        self.offset + self.len
    }
}

#[must_use]
pub fn tokens(tree: &SyntaxTree) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    for node in tree {
        if let RefNode::Locate(loc) = node {
            if let Some(text) = tree.get_str(loc) {
                if text.is_empty() || text.chars().all(char::is_whitespace) {
                    continue;
                }
                // Comments are surfaced as Locate leaves too; treat them as
                // inter-token trivia (the formatter reconstructs them from
                // src[prev..next]), not as real tokens.
                if is_comment_text(text) {
                    continue;
                }
                out.push(Token {
                    offset: loc.offset,
                    len: text.len(),
                    text,
                });
            }
        }
    }
    out.sort_by_key(|t| t.offset);
    out.dedup_by_key(|t| (t.offset, t.len));
    out
}

fn is_comment_text(s: &str) -> bool {
    let trimmed = s.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*")
}

#[derive(Debug, thiserror::Error)]
pub enum RoundTripError {
    #[error("token at offset {offset} has text {token:?} but preprocessed source has {actual:?}")]
    TokenMismatch {
        offset: usize,
        token: String,
        actual: String,
    },
    #[error(
        "reconstructed ({reconstructed} bytes) differs from preprocessed source ({original} bytes)"
    )]
    LengthMismatch {
        reconstructed: usize,
        original: usize,
    },
}

/// Reconstruct the preprocessed source from tokens + inter-token trivia.
/// Returns `Ok` iff the reconstruction equals the preprocessed source
/// byte-for-byte. Pass `parsed.text` as `text`.
pub fn assert_roundtrip(text: &str, tree: &SyntaxTree) -> Result<(), RoundTripError> {
    let toks = tokens(tree);
    let mut reconstructed = String::with_capacity(text.len());
    let mut cursor: usize = 0;
    for t in &toks {
        if t.offset < cursor {
            continue;
        }
        reconstructed.push_str(&text[cursor..t.offset]);
        let actual = &text[t.offset..t.offset + t.len];
        if actual != t.text {
            return Err(RoundTripError::TokenMismatch {
                offset: t.offset,
                token: t.text.to_owned(),
                actual: actual.to_owned(),
            });
        }
        reconstructed.push_str(t.text);
        cursor = t.end();
    }
    reconstructed.push_str(&text[cursor..]);
    if reconstructed.len() != text.len() || reconstructed != text {
        return Err(RoundTripError::LengthMismatch {
            reconstructed: reconstructed.len(),
            original: text.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn parse_ok(src: &str) -> Parsed {
        parse(src, &PathBuf::from("test.sv")).expect("parse ok")
    }

    #[test]
    fn parses_empty_module() {
        let src = "module m; endmodule\n";
        let p = parse_ok(src);
        let toks = tokens(&p.tree);
        assert!(!toks.is_empty());
        assert_eq!(toks.first().map(|t| t.text), Some("module"));
        assert!(toks.iter().any(|t| t.text == "endmodule"));
    }

    #[test]
    fn roundtrip_preserves_source() {
        let src = "module m;\n  initial begin\n    $display(\"hi\");\n  end\nendmodule\n";
        let p = parse_ok(src);
        assert_roundtrip(&p.text, &p.tree).expect("round-trip");
    }

    #[test]
    fn roundtrip_preserves_comments() {
        let src = "// leading\nmodule m; // inline\n  /* block */\nendmodule\n";
        let p = parse_ok(src);
        assert_roundtrip(&p.text, &p.tree).expect("round-trip");
    }

    #[test]
    fn roundtrip_handles_attributes() {
        let src = "(* full_case *)\nmodule m;\nendmodule\n";
        let p = parse_ok(src);
        assert_roundtrip(&p.text, &p.tree).expect("attributes round-trip");
    }
}
