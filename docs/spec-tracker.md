# vuff feature tracker

Living index of the SystemVerilog language surface we format, grouped by
IEEE 1800-2017 Annex A. Status values:

- `todo` — no rule yet; current builder falls through verbatim
- `wip` — partial rule; some shapes covered, others not
- `done` — full rule + goldens
- `skip-v0.1` — explicitly deferred per plan

When you touch a row: (1) bump status, (2) list the owning module file under
`crates/vuff_sv_formatter/src/`, (3) cite the golden(s) proving it, and
(4) keep a short "known gaps" note if `wip`. Never mark a row `done` unless
every listed golden passes **and** is exercised by both default and at least
one non-default config.

Spec reference: `docs/spec/ieee1800-2017.pdf` (Annex A is the BNF; Clause 3+
is the prose). sv-parser's `RefNode` variants are already 1:1 with Annex A
productions — use those as the dispatch keys.

## How to add a feature

1. Pick a row with status `todo` (prefer lowest Annex A number).
2. Add goldens covering: the smallest valid shape, one "common but messy"
   shape, and one that would trigger a wrap at `line_width = 60`. Use
   `// config:` and `// xfail:` directives in the golden header.
3. Implement the rule in its owning module file (see layout below).
4. Run `cargo test -p vuff_sv_formatter` — all 29+ goldens must still
   pass. Bump the row's status.

## Module layout (target, ruff-shaped)

```
crates/vuff_sv_formatter/src/
  lib.rs              format_source entry point
  context.rs          FormatCtx (opts + source + comments + indent state)
  format_ext.rs       Format trait + FormatNodeRule dispatch
  verbatim.rs         passthrough for todo-status nodes
  comments/           comment placement side-channel
  tokens/             spacing, delimiter, keyword tables
  source_text/        Annex A.1 — file root, descriptions
  module/             Annex A.1.3, A.1.4 — module decls, ports, parameters
  interface/          Annex A.1.5 — interface decls (skip-v0.1)
  program/            Annex A.1.6 — program decls (skip-v0.1)
  class/              Annex A.1.9 — class decls (skip-v0.1)
  pkg/                Annex A.1.10 — package items (skip-v0.1)
  decl/               Annex A.2 — net/variable/parameter declarations
  instantiation/      Annex A.4 — gate + module instantiation
  always/             Annex A.6.2 — always_ff / always_comb / always_latch
  stmt/               Annex A.6.3–A.6.11 — statements
  expr/               Annex A.8 — expressions
  attribute/          Annex A.9.1 — (* ... *)
```

Skip-v0.1 directories exist as stubs only; their nodes hit `verbatim.rs`.

## Annex A.1 — Source text

| Production | Node (sv-parser) | Status | Owner | Goldens |
|---|---|---|---|---|
| A.1.1 library text | `LibraryText` | skip-v0.1 | — | — |
| A.1.2 source text | `SourceText`, `Description` | done | `source_text/root.rs` | 01, 02, 30, 31, 32, 34, 35 |
| A.1.3 module parameters | `ParameterPortList`, `ParameterDeclaration` | done | `list/param_port_list.rs` | 13, 42, 130, 131, 132 |
| A.1.3 module ports | `ListOfPorts`, `ListOfPortDeclarations`, `NonansiPortDeclaration`, `AnsiPortDeclaration` | done | `list/port_paren.rs`, `list/port_align.rs`, `list/port_align_render.rs` | 12, 40, 41, 42, 43, 87, 88, 89, 90, 91 |
| A.1.4 module items | `ModuleItem`, `ModuleOrGenerateItem` | done | (pass-through tokens) | 116, 117 |
| A.1.4 module declaration | `ModuleDeclarationAnsi`, `ModuleDeclarationNonansi` | done | `module/module_declaration.rs` | 01, 02, 03, 04, 30, 31, 32, 33, 34, 35 |
| A.1.5 interface declaration | `InterfaceDeclaration*` | skip-v0.1 | — | — |
| A.1.6 program declaration | `ProgramDeclaration*` | skip-v0.1 | — | — |
| A.1.7 checker declaration | `CheckerDeclaration` | skip-v0.1 | — | — |
| A.1.8 class | `ClassDeclaration`, `ClassItem` | skip-v0.1 | — | — |
| A.1.9 configuration source | `ConfigDeclaration` | skip-v0.1 | — | — |
| A.1.10 package | `PackageDeclaration`, `PackageItem` | done | `indent_map.rs` | 133, 135 |
| A.1.11 preprocessor | (handled by sv-parser preprocessor) | done | — | (implicit) |

## Annex A.2 — Declarations

| Production | Node | Status | Owner | Goldens |
|---|---|---|---|---|
| A.2.1.1 module parameter | `LocalParameterDeclaration`, `ParameterDeclaration` | done | (pass-through tokens) | 13, 115 |
| A.2.1.2 port declarations | `InputDeclaration`, `OutputDeclaration`, `InoutDeclaration`, `RefDeclaration` | done | `list/port_align.rs` | 12, 40, 41, 42, 43 |
| A.2.1.3 type declarations | `TypeDeclaration` | done | (pass-through tokens) | 104 |
| A.2.1.3 net-type declaration | `NetTypeDeclaration` | skip-v0.1 | — | — |
| A.2.2 net & variable types | `NetDeclaration`, `DataDeclaration` | done | (pass-through tokens) | 100, 101, 102, 103, 104, 105 |
| A.2.2 package import | `PackageImportDeclaration` | done | `tokens/spacing.rs` | 134 |
| A.2.3 declaration lists | `ListOfVariableDeclAssignments`, `ListOfPortIdentifiers` | done | (pass-through tokens) | 102, 13 |
| A.2.4 declaration assignments | `VariableDeclAssignment`, `NetDeclAssignment` | done | (pass-through tokens) | 103 |
| A.2.5 declaration ranges | `UnpackedDimension`, `PackedDimension` | done | (pass-through tokens) | 28, 101, 112 |
| A.2.6 function declarations | `FunctionDeclaration`, `FunctionBodyDeclaration` | done | `indent_map.rs` | 119, 122 |
| A.2.7 task declarations | `TaskDeclaration` | done | `indent_map.rs` | 120 |
| A.2.8 block item declarations | `BlockItemDeclaration` | done | `indent_map.rs` | 121 |
| A.2.9 interface declarations | modports etc. | skip-v0.1 | — | — |
| A.2.10 assertion declarations | `SequenceDeclaration`, `PropertyDeclaration` | skip-v0.1 | — | — |
| A.2.11 covergroup | `CovergroupDeclaration` | skip-v0.1 | — | — |

## Annex A.3–A.5 — Primitives, instantiation, UDP

| Production | Node | Status | Owner | Goldens |
|---|---|---|---|---|
| A.3 primitive instances | gate-level primitives | done | (pass-through tokens) | 114 |
| A.4.1 module instantiation | `ModuleInstantiation`, `HierarchicalInstance`, `NamedPortConnection`, `OrderedPortConnection` | done | `list/instance_paren.rs`, `list/param_assign.rs`, `list/inst_port_list.rs` | 24 (xfail), 116, 117, 118, 143, 144, 145, 146 |
| A.4.1 parameter override | `NamedParameterAssignment`, `OrderedParameterAssignment` | done | `list/param_assign.rs` | 117 |
| A.4.2 generate | `GenerateRegion`, `LoopGenerateConstruct`, `IfGenerateConstruct` | done | `tokens/delimiters.rs`, `indent_map.rs` | 11, 123, 124, 125, 126 |
| A.4.2 case-generate | `CaseGenerateConstruct` | done | `indent_map.rs`, `stmt/reset_mask.rs` | 140 |
| A.5 UDP | `UdpDeclaration` | skip-v0.1 | — | — |

## Annex A.6 — Behavioral statements

| Production | Node | Status | Owner | Goldens |
|---|---|---|---|---|
| A.6.1 continuous assignment | `ContinuousAssign` | done | (pass-through tokens) | 96, 97, 98, 99 |
| A.6.1 net alias | `NetAlias` | skip-v0.1 | — | — |
| A.6.2 procedural blocks | `AlwaysConstruct`, `InitialConstruct`, `FinalConstruct` | done | `indent_map.rs` | 10, 21, 51, 52 |
| A.6.2 procedural assignments | `BlockingAssignment`, `NonblockingAssignment`, `ProceduralContinuousAssignment` | done | `stmt/boundaries.rs` | 10, 11, 16, 51 |
| A.6.3 parallel/sequential blocks | `SeqBlock`, `ParBlock` | done | `indent_map.rs`, `stmt/seq_block.rs` | 08, 09, 44, 45, 46, 47, 53 |
| A.6.4 statements | `StatementOrNull`, `Statement` | done | `stmt/boundaries.rs`, `indent_map.rs` | all |
| A.6.5 timing control | `DelayControl`, `EventControl`, `CycleDelay` | done | (pass-through tokens) | 10, 136 |
| A.6.6 conditional | `ConditionalStatement`, `UniquePriority`, `CondPredicate` | done | `indent_map.rs` | 16, 51, 54 |
| A.6.7 case | `CaseStatementNormal`, `CaseStatementInside`, `CaseStatementMatches`, `CaseItemNondefault`, `CaseItemDefault` | done | `indent_map.rs` | 11, 29, 48, 53 |
| A.6.8 patterns | `PatternAny`, `PatternIdentifier`, `PatternConcat` | skip-v0.1 | — | — |
| A.6.8 looping | `LoopStatement`, `ForInitialization`, `ForStep` | done | `indent_map.rs` | 52 |
| A.6.9 subroutine calls | `SubroutineCallStatement`, `SystemTfCall`, `TfCall` | done | (pass-through tokens) | 106, 107, 109 |
| A.6.10 assertion stmts | `ConcurrentAssertionItem`, `ImmediateAssertionStatement` | skip-v0.1 | — | — |
| A.6.11 clocking | `ClockingDeclaration` | skip-v0.1 | — | — |
| A.6.12 randsequence | `RandsequenceStatement` | skip-v0.1 | — | — |

## Annex A.8 — Expressions

| Production | Node | Status | Owner | Goldens |
|---|---|---|---|---|
| A.8.1 concatenation | `Concatenation`, `MultipleConcatenation` | done | (pass-through tokens) | 110 |
| A.8.1 streaming concatenation | `StreamingConcatenation` | done | `expr/streaming.rs` | 142 |
| A.8.2 subroutine calls | `SystemTfCall`, `TfCall`, `MethodCall` | done | (pass-through tokens) | 106, 107, 108, 109 |
| A.8.3 expressions | `Expression`, `BinaryExpression`, `UnaryExpression`, `IncOrDecExpression` | done | `tokens/spacing.rs`, `expr/binary.rs`, `expr/unary.rs` | 01, 07, 137, 138, 139 |
| A.8.3 conditional expression | `ConditionalExpression` | done | `expr/conditional.rs` | 26, 27, 28, 48, 49, 50 |
| A.8.3 inside expression | `InsideExpression` | done | (pass-through tokens) | 113 |
| A.8.4 primaries | `Primary`, `PrimaryLiteral` | done | (pass-through tokens) | 111 |
| A.8.4 empty queue | `EmptyQueue` | done | (pass-through tokens) | 141 |
| A.8.5 expression left-side | `VariableLvalue`, `NetLvalue` | done | (pass-through tokens) | 110 |
| A.8.6 operators | `UnaryOperator`, `BinaryOperator`, `IncOrDecOperator` | done | `tokens/spacing.rs` | 01, 137, 138, 139 |
| A.8.7 numbers | `IntegralNumber`, `RealNumber`, `DecimalNumber` | done | (pass-through tokens) | 01 |
| A.8.8 strings | `StringLiteral` | done | (pass-through tokens) | — |

## Annex A.9 — General

| Production | Node | Status | Owner | Goldens |
|---|---|---|---|---|
| A.9.1 attributes | `AttributeInstance`, `AttrSpec` | done | `attribute/spans.rs` | 18, 19, 20, 22, 23, 24, 25, 36, 37, 38, 39 |
| A.9.2 comments | `Comment` | done | `tokens/trivia.rs` | 14, 15, 127, 128, 129 |
| A.9.3 identifiers | `Identifier`, `HierarchicalIdentifier`, `EscapedIdentifier` | done | (pass-through tokens) | — |

## Whitespace & layout policies (cross-cutting)

| Policy | Status | Owner | Goldens |
|---|---|---|---|
| Indent width (spaces) | done | `context.rs` | 02, 03 |
| Indent style (tabs) | done | `context.rs` | 04 |
| Blank-line collapse (max 1) | done | `tokens/trivia.rs` | 05 |
| Trailing whitespace strip | done | `tokens/trivia.rs` | 06 |
| Trailing blank lines strip | done | `tokens/trivia.rs` | 07 |
| Exactly one trailing newline | done | `lib.rs` | 07 |
| `begin_style = k_and_r` | done | `stmt/seq_block.rs` | 08, 44, 45, 46 |
| `begin_style = allman` | done | `stmt/seq_block.rs` | 09, 47 |
| Block label `: name` suppresses continuation indent | done | `verbatim.rs` (label_pending) | 44, 45, 47 |
| Port list one-per-line layout (newline-triggered) | done | `list/port_align_render.rs`, `list/param_port_list.rs` | 13, 130, 131, 132 |
| Space before port-list `(` | done | `list/port_paren.rs` | 12, 40, 41, 42 |
| Newline-triggered wrap — instantiation port lists | done | `list/inst_port_list.rs` | 143, 144, 145, 146, 147 |
| Newline-triggered wrap — generic delimited groups (`(…)`/`{…}`/`[…]`) | done | `list/wrap_mask.rs`, `verbatim.rs` | 148, 149, 150, 151 |
| Auto-wrap on line-width overflow | skip-v0.1 | by design — formatter never wraps unless user inserts a newline; long lines are the user's responsibility | — |
| Statement continuation indent | done | `stmt/boundaries.rs`, `indent_map.rs` | all |
| Implicit-begin body indent (if/for/while/always without begin) | done | `indent_map.rs` | 51, 52, 54 |
| Ternary multi-line layout | done | `expr/conditional.rs` | 26, 27, 49, 50 |
| Multi-line attribute layout | done | `attribute/spans.rs` | 22, 25, 39 |
| Comment attachment (leading / trailing / dangling) | done | `tokens/trivia.rs` | 14, 15 |

## Out of scope for v0.1 (explicit)

- Generate regions & blocks
- SVA: `property`, `sequence`, `assert property`, `cover property`
- `interface`, `package`, `program`, `checker`
- `class`, `covergroup`
- `specify` blocks, UDPs
- `config` / library source
- Macro expansion inside expressions
- Format-string awareness for `$display` / `$sformatf`

These have `skip-v0.1` in the tables above and route through `verbatim.rs`.
