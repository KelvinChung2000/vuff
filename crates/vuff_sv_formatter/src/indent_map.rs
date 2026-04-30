//! Per-token structural indent depth, computed once by walking the CST.
//! Combined with the token-level block depth tracked in `verbatim`
//! (brackets, `begin`/`end`, `case`/`endcase`, `fork`/`join*`) to form the
//! total depth used for re-indenting new lines.
//!
//! CST contributions here cover structure that has no literal open/close
//! tokens: module-body items and the implicit-begin body of
//! `if` / `for` / `while` / `always*` / `initial` / `final`.

use vuff_sv_ast::{
    DirectiveDetail, FunctionStatementOrNull, NodeEvent, Parsed, RefNode, Statement, StatementItem,
    StatementOrNull, SyntaxTree, Token,
};

/// `is_directive_start[i] == true` when token `i` is the first token of
/// a surviving preprocessor directive line (`\`define`, `\`timescale`,
/// `\`include`, …). The verbatim engine uses this to reset
/// `in_statement` at the directive boundary so a chained sequence like
/// `\`timescale 1 ns / 1 ps` followed by `\`define X 0` doesn't carry
/// the prior continuation bump into the new directive.
pub(crate) fn directive_start_mask(parsed: &Parsed, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut out = vec![false; tokens.len()];
    if tokens.is_empty() {
        return out;
    }
    let offsets: Vec<usize> = tokens.iter().map(|t| t.offset).collect();
    for d in parsed.tree.directives() {
        if d.original_path != parsed.original_path {
            continue;
        }
        let Some(pp) = d.pp_range else {
            continue;
        };
        let idx = offsets.partition_point(|&o| o < pp.begin);
        if idx < tokens.len() && tokens[idx].offset < pp.end {
            out[idx] = true;
        }
    }
    out
}

/// Number of `\`ifdef`/`\`ifndef`/etc. branch bodies enclosing each token.
/// Used to indent active code one level deeper per enclosing chain so the
/// chain's keyword can sit one level outside its body. Tokens outside any
/// chain get `0`.
pub(crate) fn chain_depth_map(parsed: &Parsed, tokens: &[Token<'_>]) -> Vec<u32> {
    let intervals = collect_branch_body_intervals(parsed);
    if intervals.is_empty() {
        return vec![0u32; tokens.len()];
    }
    tokens
        .iter()
        .map(|t| {
            let Some(orig) = parsed.origin_in_original(t.offset) else {
                return 0;
            };
            chain_depth_at(&intervals, orig)
        })
        .collect()
}

/// Branch-body byte ranges (in original-source coordinates) for every
/// `\`ifdef` / `\`ifndef` / `\`elsif` / `\`else` branch, regardless of
/// whether it was taken.
pub(crate) fn collect_branch_body_intervals(parsed: &Parsed) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for d in parsed.tree.directives() {
        if d.original_path != parsed.original_path {
            continue;
        }
        let DirectiveDetail::IfdefChain(ref chain) = d.detail else {
            continue;
        };
        for b in &chain.branches {
            if let Some(body) = &b.body_original_range {
                out.push((body.begin, body.end));
            }
        }
    }
    out
}

/// Count of branch-body intervals containing `pos` (linear; the count is
/// always small in practice).
pub(crate) fn chain_depth_at(intervals: &[(usize, usize)], pos: usize) -> u32 {
    u32::try_from(
        intervals
            .iter()
            .filter(|&&(b, e)| b <= pos && pos < e)
            .count(),
    )
    .unwrap_or(0)
}

/// Per-frame record so Leave dedents the exact amount Enter bumped.
/// `kind` is `None` for `Locate` frames (they never bump).
struct Frame {
    delta: u32,
    kind: Option<ParentKind>,
}

pub(crate) fn cst_depth_map(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<u32> {
    let mut depths = vec![0u32; tokens.len()];
    let mut depth: u32 = 0;
    let mut stack: Vec<Frame> = Vec::new();

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::Locate(loc)) => {
                if let Ok(idx) = tokens.binary_search_by_key(&loc.offset, |t| t.offset) {
                    depths[idx] = depth;
                }
                stack.push(Frame {
                    delta: 0,
                    kind: None,
                });
            }
            NodeEvent::Enter(node) => {
                let parent = stack.iter().rev().find_map(|f| f.kind.as_ref().copied());
                // True when the nearest enclosing fn-task-body or seq-block
                // is the fn-task-body — i.e. we're at the outer body level
                // of a function/task with no intervening begin/end. Used to
                // give that outer level its implicit body indent.
                let outer_fn_task = stack
                    .iter()
                    .rev()
                    .find_map(|f| match f.kind {
                        Some(ParentKind::SeqOrParBlock) => Some(false),
                        Some(ParentKind::FunctionTaskBody) => Some(true),
                        _ => None,
                    })
                    .unwrap_or(false);
                let in_package = stack
                    .iter()
                    .any(|f| matches!(f.kind, Some(ParentKind::PackageDecl)));
                let bump = decide_bump(&node, parent, outer_fn_task, in_package);
                depth += bump;
                stack.push(Frame {
                    delta: bump,
                    kind: Some(kind_of(&node)),
                });
            }
            NodeEvent::Leave(_) => {
                if let Some(f) = stack.pop() {
                    depth -= f.delta;
                }
            }
        }
    }
    depths
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParentKind {
    AlwaysLike,           // AlwaysConstruct / InitialConstruct / FinalConstruct
    ConditionalStatement, // if / else if / else
    LoopStatement,        // for/while/forever/repeat/do-while/foreach
    ProceduralTimingCtl,  // @(...)
    SeqOrParBlock,        // begin...end / fork...join
    CaseItem,             // case arm
    FunctionTaskBody,     // function...endfunction / task...endtask body
    PackageDecl,          // package...endpackage — distinguishes a real
    // package body from a top-level Description::PackageItem
    // (a stray declaration at file scope).
    // Other node kinds we don't care about, but they still shadow further
    // ancestors in the immediate-parent search so transitive structural
    // relationships don't accidentally double-bump indent.
    Other,
}

fn kind_of(node: &RefNode<'_>) -> ParentKind {
    match node {
        RefNode::AlwaysConstruct(_) | RefNode::InitialConstruct(_) | RefNode::FinalConstruct(_) => {
            ParentKind::AlwaysLike
        }
        RefNode::ConditionalStatement(_) => ParentKind::ConditionalStatement,
        RefNode::LoopStatement(_)
        | RefNode::LoopStatementForever(_)
        | RefNode::LoopStatementRepeat(_)
        | RefNode::LoopStatementWhile(_)
        | RefNode::LoopStatementFor(_)
        | RefNode::LoopStatementDoWhile(_)
        | RefNode::LoopStatementForeach(_) => ParentKind::LoopStatement,
        RefNode::ProceduralTimingControlStatement(_) => ParentKind::ProceduralTimingCtl,
        RefNode::SeqBlock(_) | RefNode::ParBlock(_) => ParentKind::SeqOrParBlock,
        RefNode::FunctionDeclaration(_) | RefNode::TaskDeclaration(_) => {
            ParentKind::FunctionTaskBody
        }
        RefNode::PackageDeclaration(_) => ParentKind::PackageDecl,
        RefNode::CaseItemNondefault(_)
        | RefNode::CaseItemDefault(_)
        | RefNode::CasePatternItemNondefault(_)
        | RefNode::CaseInsideItemNondefault(_) => ParentKind::CaseItem,
        _ => ParentKind::Other,
    }
}

fn decide_bump(
    node: &RefNode<'_>,
    parent: Option<ParentKind>,
    outer_fn_task: bool,
    in_package: bool,
) -> u32 {
    match node {
        // Module-body / case-arm / generate-body items. Each one sits
        // one level deeper than its surrounding container.
        RefNode::NonPortModuleItem(_)
        | RefNode::ModuleItem(_)
        | RefNode::CaseItemNondefault(_)
        | RefNode::CaseItemDefault(_)
        | RefNode::CasePatternItemNondefault(_)
        | RefNode::CaseInsideItemNondefault(_)
        | RefNode::CaseGenerateItemNondefault(_)
        | RefNode::CaseGenerateItemDefault(_)
        | RefNode::GenerateItem(_) => 1,
        // Package body items only bump when we're actually inside a
        // `package … endpackage`. At top level, sv-parser also wraps
        // stray declarations as `Description::PackageItem`, but those
        // sit at the source-text root with no enclosing indent.
        RefNode::PackageItem(_) if in_package => 1,
        // Block-item declarations sit inside begin/end and inside
        // function/task bodies. The immediate parent is typically a
        // transparent wrapper (FunctionBodyDeclaration, TfItemDeclaration),
        // so we accept either a direct SeqOrParBlock parent OR an outer
        // function/task body context.
        RefNode::BlockItemDeclaration(_) => {
            u32::from(matches!(parent, Some(ParentKind::SeqOrParBlock)) || outer_fn_task)
        }
        // Tf-item declarations (function/task body declarations and port
        // declarations on non-ANSI subroutines) wrap BlockItemDeclaration —
        // only bump on the outer one to avoid double counting.
        RefNode::TfItemDeclaration(_) if outer_fn_task => 1,
        RefNode::Statement(s) => stmt_bump(parent, s),
        RefNode::StatementOrNull(s) => son_bump(parent, s, outer_fn_task),
        // Function-body statements wrap a Statement but appear under
        // FunctionStatementOrNull, not StatementOrNull. The inner Statement
        // sees parent=Other (transparent wrapper) and won't double-bump.
        RefNode::FunctionStatementOrNull(s) if outer_fn_task => fson_bump(s),
        _ => 0,
    }
}

/// A `Statement` bumps depth when it's the body slot of a construct that
/// has no block of its own (if / for / while / always* / initial / final /
/// @(...)), UNLESS the body itself is a `begin…end` / `fork…join` block
/// (those handle their depth via their own keyword tokens) or a
/// `ProceduralTimingControlStatement` (which in turn may wrap a block —
/// the inner `StatementOrNull` will decide).
fn stmt_bump(parent: Option<ParentKind>, s: &Statement) -> u32 {
    if wraps_block(&s.nodes.2) || wraps_timing_ctl(&s.nodes.2) {
        return 0;
    }
    match parent {
        Some(
            ParentKind::AlwaysLike
            | ParentKind::ConditionalStatement
            | ParentKind::LoopStatement
            | ParentKind::ProceduralTimingCtl,
        ) => 1,
        _ => 0,
    }
}

fn fson_bump(s: &FunctionStatementOrNull) -> u32 {
    let inner = match s {
        FunctionStatementOrNull::Statement(fs) => &fs.nodes.0,
        FunctionStatementOrNull::Attribute(_) => return 0,
    };
    let content_is_block_or_timing =
        wraps_block(&inner.nodes.2) || wraps_timing_ctl(&inner.nodes.2);
    u32::from(!content_is_block_or_timing)
}

fn son_bump(parent: Option<ParentKind>, s: &StatementOrNull, outer_fn_task: bool) -> u32 {
    let inner = match s {
        StatementOrNull::Statement(s) => s,
        StatementOrNull::Attribute(_) => return 0,
    };
    let content_is_block_or_timing =
        wraps_block(&inner.nodes.2) || wraps_timing_ctl(&inner.nodes.2);
    match parent {
        // Inside a SeqBlock / ParBlock, every body-StatementOrNull is a
        // proper child and therefore bumps — the surrounding begin/end
        // (or fork/join) sit at the block's *parent* depth because they
        // are siblings of these children, not children of them. The
        // content-is-block exclusion does not apply here because nested
        // blocks genuinely sit one level deeper than their enclosing
        // block.
        Some(ParentKind::SeqOrParBlock) => 1,
        // For body slots of control-flow constructs (if / for / while /
        // always / initial / final / @(...)) we want an *implicit* +1
        // only when there's no explicit block — i.e., when the body
        // isn't itself a SeqBlock / ParBlock / ProceduralTimingControl.
        Some(
            ParentKind::AlwaysLike
            | ParentKind::ConditionalStatement
            | ParentKind::LoopStatement
            | ParentKind::ProceduralTimingCtl,
        ) => u32::from(!content_is_block_or_timing),
        // Task bodies use plain StatementOrNull (not FunctionStatementOrNull)
        // and reach this fn through a transparent wrapper. When we're at
        // the outer body level of a function/task, treat it like
        // fson_bump: bump unless the body itself is a block/timing.
        _ if outer_fn_task => u32::from(!content_is_block_or_timing),
        _ => 0,
    }
}

fn wraps_block(item: &StatementItem) -> bool {
    matches!(
        item,
        StatementItem::SeqBlock(_) | StatementItem::ParBlock(_)
    )
}

fn wraps_timing_ctl(item: &StatementItem) -> bool {
    matches!(item, StatementItem::ProceduralTimingControlStatement(_))
}
