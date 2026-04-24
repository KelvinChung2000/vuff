//! Per-token structural indent depth, computed once by walking the CST.
//! Combined with the token-level block depth tracked in `verbatim`
//! (brackets, `begin`/`end`, `case`/`endcase`, `fork`/`join*`) to form the
//! total depth used for re-indenting new lines.
//!
//! CST contributions here cover structure that has no literal open/close
//! tokens: module-body items and the implicit-begin body of
//! `if` / `for` / `while` / `always*` / `initial` / `final`.

use svfmt_sv_ast::{NodeEvent, RefNode, Statement, StatementItem, StatementOrNull, SyntaxTree, Token};

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
                stack.push(Frame { delta: 0, kind: None });
            }
            NodeEvent::Enter(node) => {
                let parent = stack.iter().rev().find_map(|f| f.kind.as_ref().copied());
                let bump = decide_bump(&node, parent);
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
    // Other node kinds we don't need to match on.
    Other,
}

fn kind_of(node: &RefNode<'_>) -> ParentKind {
    match node {
        RefNode::AlwaysConstruct(_)
        | RefNode::InitialConstruct(_)
        | RefNode::FinalConstruct(_) => ParentKind::AlwaysLike,
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
        RefNode::CaseItemNondefault(_)
        | RefNode::CaseItemDefault(_)
        | RefNode::CasePatternItemNondefault(_)
        | RefNode::CaseInsideItemNondefault(_) => ParentKind::CaseItem,
        _ => ParentKind::Other,
    }
}

fn decide_bump(node: &RefNode<'_>, parent: Option<ParentKind>) -> u32 {
    match node {
        // Module-body / case-arm / generate-body items. Each one sits
        // one level deeper than its surrounding container.
        RefNode::NonPortModuleItem(_)
        | RefNode::ModuleItem(_)
        | RefNode::CaseItemNondefault(_)
        | RefNode::CaseItemDefault(_)
        | RefNode::CasePatternItemNondefault(_)
        | RefNode::CaseInsideItemNondefault(_)
        | RefNode::GenerateItem(_) => 1,
        RefNode::Statement(s) => stmt_bump(parent, s),
        RefNode::StatementOrNull(s) => son_bump(parent, s),
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

fn son_bump(parent: Option<ParentKind>, s: &StatementOrNull) -> u32 {
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
        _ => 0,
    }
}

fn wraps_block(item: &StatementItem) -> bool {
    matches!(item, StatementItem::SeqBlock(_) | StatementItem::ParBlock(_))
}

fn wraps_timing_ctl(item: &StatementItem) -> bool {
    matches!(item, StatementItem::ProceduralTimingControlStatement(_))
}
