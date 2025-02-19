//! This module provides a pass that removes parts of MIR that are no longer relevant after
//! analysis phase and borrowck. In particular, it removes false edges, user type annotations and
//! replaces following statements with [`Nop`]s:
//!
//!   - [`AscribeUserType`]
//!   - [`FakeRead`]
//!   - [`Assign`] statements with a [`Fake`] borrow
//!   - [`Coverage`] statements of kind [`BlockMarker`] or [`SpanMarker`]
//!
//! [`AscribeUserType`]: rustc_middle::mir::StatementKind::AscribeUserType
//! [`Assign`]: rustc_middle::mir::StatementKind::Assign
//! [`FakeRead`]: rustc_middle::mir::StatementKind::FakeRead
//! [`Nop`]: rustc_middle::mir::StatementKind::Nop
//! [`Fake`]: rustc_middle::mir::BorrowKind::Fake
//! [`Coverage`]: rustc_middle::mir::StatementKind::Coverage
//! [`BlockMarker`]: rustc_middle::mir::coverage::CoverageKind::BlockMarker
//! [`SpanMarker`]: rustc_middle::mir::coverage::CoverageKind::SpanMarker

use crate::MirPass;
use rustc_middle::mir::coverage::CoverageKind;
use rustc_middle::mir::{Body, BorrowKind, Coverage, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::TyCtxt;

pub struct CleanupPostBorrowck;

impl<'tcx> MirPass<'tcx> for CleanupPostBorrowck {
    fn run_pass(&self, _tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        for basic_block in body.basic_blocks.as_mut() {
            for statement in basic_block.statements.iter_mut() {
                match statement.kind {
                    StatementKind::AscribeUserType(..)
                    | StatementKind::Assign(box (_, Rvalue::Ref(_, BorrowKind::Fake, _)))
                    | StatementKind::Coverage(box Coverage {
                        // These kinds of coverage statements are markers inserted during
                        // MIR building, and are not needed after InstrumentCoverage.
                        kind: CoverageKind::BlockMarker { .. } | CoverageKind::SpanMarker { .. },
                        ..
                    })
                    | StatementKind::FakeRead(..) => statement.make_nop(),
                    _ => (),
                }
            }
            let terminator = basic_block.terminator_mut();
            match terminator.kind {
                TerminatorKind::FalseEdge { real_target, .. }
                | TerminatorKind::FalseUnwind { real_target, .. } => {
                    terminator.kind = TerminatorKind::Goto { target: real_target };
                }
                _ => {}
            }
        }

        body.user_type_annotations.raw.clear();

        for decl in &mut body.local_decls {
            decl.user_ty = None;
        }
    }
}
