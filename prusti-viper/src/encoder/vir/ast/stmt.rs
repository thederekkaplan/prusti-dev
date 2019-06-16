// © 2019, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::super::borrows::{Borrow, DAG as ReborrowingDAG};
use super::super::cfg::CfgBlockIndex;
use encoder::vir::ast::*;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Comment(String),
    Label(String),
    Inhale(Expr, FoldingBehaviour),
    Exhale(Expr, Position),
    Assert(Expr, FoldingBehaviour, Position),
    /// MethodCall: method_name, args, targets
    MethodCall(String, Vec<Expr>, Vec<LocalVar>),
    Assign(Expr, Expr, AssignKind),
    /// Fold statement: predicate name, predicate args, perm_amount, variant of enum, position.
    Fold(String, Vec<Expr>, PermAmount, MaybeEnumVariantIndex, Position),
    /// Unfold statement: predicate name, predicate args, perm_amount, variant of enum.
    Unfold(String, Vec<Expr>, PermAmount, MaybeEnumVariantIndex),
    /// Obtain: conjunction of Expr::PredicateAccessPredicate or Expr::FieldAccessPredicate
    /// They will be used by the fold/unfold algorithm
    Obtain(Expr, Position),
    /// WeakObtain: conjunction of Expr::PredicateAccessPredicate or Expr::FieldAccessPredicate
    /// They will be used by the fold/unfold algorithm
    #[deprecated]
    WeakObtain(Expr),
    /// Havoc: used for emptying the fold/unfold state
    #[deprecated]
    Havoc,
    /// Mark a CFG point in which all current permissions are framed out
    /// They will be used by the fold/unfold algorithm
    BeginFrame,
    /// Mark a CFG point in which all the permissions of a corresponding `BeginFrame` are framed in
    /// They will be used by the fold/unfold algorithm
    EndFrame,
    /// Move permissions from a place to another.
    /// This is used to restore permissions in the fold/unfold state when a loan expires.
    ///
    /// The last argument indicates if the transfer is unchecked. Unchecked transfer is used for
    /// encoding shared borrows which can be dangling and, therefore, we cannot use the safety
    /// checks.
    TransferPerm(Expr, Expr, bool),
    /// Package a Magic Wand
    /// Arguments: the magic wand, the package statement's body, the
    /// label just before the statement, and ghost variables used inside
    /// the package statement.
    PackageMagicWand(Expr, Vec<Stmt>, String, Vec<LocalVar>, Position),
    /// Apply a Magic Wand.
    /// Arguments: the magic wand.
    ApplyMagicWand(Expr, Position),
    /// Expire borrows given in the reborrowing DAG.
    ExpireBorrows(ReborrowingDAG),
    /// An `if` statement: the guard and the 'then' branch.
    If(Expr, Vec<Stmt>),
}

/// What folding behaviour should be used?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldingBehaviour {
    /// Use `fold` and `unfold` statements.
    Stmt,
    /// Use `unfolding` expressions.
    Expr,
    /// Should not require changes in folding.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignKind {
    /// Encodes a Rust copy.
    /// This assignment can be used iff the Viper type of the `lhs` and `rhs` is *not* Ref.
    Copy,
    /// Encodes a Rust move. The permissions in the rhs move to the `lhs`.
    /// This assignment can be used iff the Viper type of the `lhs` and `rhs` is Ref.
    Move,
    /// Encodes the initialization of a mutable borrow.
    /// The permissions in the `rhs` move to the `lhs`, but they can be restored when the borrow dies.
    MutableBorrow(Borrow),
    /// Encodes the initialization of a shared borrow.
    /// The permissions in the `rhs` are duplicated to the `lhs`.
    SharedBorrow(Borrow),
    /// Used to mark that the assignment is to a ghost variable and should be ignored by
    /// the fold-unfold algorithm.
    Ghost,
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Stmt::Comment(ref comment) => write!(f, "// {}", comment),
            Stmt::Label(ref label) => write!(f, "label {}", label),
            Stmt::Inhale(ref expr, ref folding) => {
                write!(f, "inhale({:?}) {}", folding, expr)
            },
            Stmt::Exhale(ref expr, _) => write!(f, "exhale {}", expr),
            Stmt::Assert(ref expr, ref folding, _) => {
                write!(f, "assert({:?}) {}", folding, expr)
            },
            Stmt::MethodCall(ref name, ref args, ref vars) => write!(
                f,
                "{} := {}({})",
                vars.iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                name,
                args.iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            Stmt::Assign(ref lhs, ref rhs, kind) => match kind {
                AssignKind::Move => write!(f, "{} := move {}", lhs, rhs),
                AssignKind::Copy => write!(f, "{} := copy {}", lhs, rhs),
                AssignKind::MutableBorrow(borrow) => {
                    write!(f, "{} := mut borrow {} // {:?}", lhs, rhs, borrow)
                }
                AssignKind::SharedBorrow(borrow) => {
                    write!(f, "{} := borrow {} // {:?}", lhs, rhs, borrow)
                }
                AssignKind::Ghost => write!(f, "{} := ghost {}", lhs, rhs),
            },

            Stmt::Fold(ref pred_name, ref args, perm, ref variant, _) => write!(
                f,
                "fold acc({}:{:?}({}), {})",
                pred_name,
                variant,
                args.iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                perm,
            ),

            Stmt::Unfold(ref pred_name, ref args, perm, ref variant) => write!(
                f,
                "unfold acc({}:{:?}({}), {})",
                pred_name,
                variant,
                args.iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                perm,
            ),

            Stmt::Obtain(ref expr, _) => write!(f, "obtain {}", expr),

            Stmt::WeakObtain(ref expr) => write!(f, "weak obtain {}", expr),

            Stmt::Havoc => write!(f, "havoc"),

            Stmt::BeginFrame => write!(f, "begin frame"),

            Stmt::EndFrame => write!(f, "end frame"),

            Stmt::TransferPerm(ref lhs, ref rhs, unchecked) => write!(
                f,
                "transfer perm {} --> {} // unchecked: {}",
                lhs, rhs, unchecked
            ),

            Stmt::PackageMagicWand(
                Expr::MagicWand(ref lhs, ref rhs, None, _),
                ref package_stmts,
                ref label,
                _vars,
                _position,
            ) => {
                writeln!(f, "package[{}] {}", label, lhs)?;
                writeln!(f, "    --* {}", rhs)?;
                write!(f, "{{")?;
                if !package_stmts.is_empty() {
                    write!(f, "\n")?;
                }
                for stmt in package_stmts.iter() {
                    writeln!(f, "    {}", stmt.to_string().replace("\n", "\n    "))?;
                }
                write!(f, "}}")
            }

            Stmt::ApplyMagicWand(Expr::MagicWand(ref lhs, ref rhs, Some(borrow), _), _) => {
                writeln!(f, "apply[{:?}] {} --* {}", borrow, lhs, rhs)
            }

            Stmt::ExpireBorrows(dag) => writeln!(f, "expire_borrows {:?}", dag),

            Stmt::If(ref guard, ref then_stmts) => {
                write!(f, "if {} {{", guard)?;
                if !then_stmts.is_empty() {
                    write!(f, "\n")?;
                }
                for stmt in then_stmts.iter() {
                    writeln!(f, "    {}", stmt.to_string().replace("\n", "\n    "))?;
                }
                write!(f, "}}")
            }

            ref x => unimplemented!("{:?}", x),
        }
    }
}

impl Stmt {
    pub fn is_comment(&self) -> bool {
        match self {
            Stmt::Comment(_) => true,
            _ => false,
        }
    }

    pub fn comment<S: ToString>(comment: S) -> Self {
        Stmt::Comment(comment.to_string())
    }

    pub fn obtain_acc(place: Expr, pos: Position) -> Self {
        assert!(!place.is_local());
        Stmt::Obtain(
            Expr::FieldAccessPredicate(box place, PermAmount::Write, pos.clone()),
            pos,
        )
    }

    pub fn obtain_pred(place: Expr, pos: Position) -> Self {
        let predicate_name = place.typed_ref_name().unwrap();
        Stmt::Obtain(
            Expr::PredicateAccessPredicate(
                predicate_name,
                box place,
                PermAmount::Write,
                pos.clone(),
            ),
            pos,
        )
    }

    pub fn fold_pred(
        place: Expr,
        perm: PermAmount,
        variant: MaybeEnumVariantIndex,
        pos: Position
    ) -> Self {
        let predicate_name = place.typed_ref_name().unwrap();
        Stmt::Fold(predicate_name, vec![place.into()], perm, variant, pos)
    }

    pub fn unfold_pred(
        place: Expr,
        perm: PermAmount,
        variant: MaybeEnumVariantIndex
    ) -> Self {
        let predicate_name = place.typed_ref_name().unwrap();
        Stmt::Unfold(predicate_name, vec![place], perm, variant)
    }

    pub fn package_magic_wand(
        lhs: Expr,
        rhs: Expr,
        stmts: Vec<Stmt>,
        label: String,
        vars: Vec<LocalVar>,
        pos: Position,
    ) -> Self {
        Stmt::PackageMagicWand(
            Expr::MagicWand(box lhs, box rhs, None, pos.clone()),
            stmts,
            label,
            vars,
            pos,
        )
    }

    pub fn apply_magic_wand(lhs: Expr, rhs: Expr, borrow: Borrow, pos: Position) -> Self {
        Stmt::ApplyMagicWand(Expr::magic_wand(lhs, rhs, Some(borrow)), pos)
    }

    pub fn pos(&self) -> Option<&Position> {
        match self {
            Stmt::PackageMagicWand(_, _, _, _, ref p) => Some(p),
            _ => None,
        }
    }

    pub fn set_pos(self, pos: Position) -> Self {
        match self {
            Stmt::PackageMagicWand(w, s, l, v, p) => Stmt::PackageMagicWand(w, s, l, v, pos),
            x => x,
        }
    }

    // Replace a Position::default() position with `pos`
    pub fn set_default_pos(self, pos: Position) -> Self {
        if self.pos().iter().any(|x| x.is_default()) {
            self.set_pos(pos)
        } else {
            self
        }
    }

    // Replace all Position::default() positions in expressions with `pos`
    pub fn set_default_expr_pos(self, pos: Position) -> Self {
        self.map_expr(|e| e.set_default_pos(pos.clone()))
    }
}

pub trait StmtFolder {
    fn fold(&mut self, e: Stmt) -> Stmt {
        match e {
            Stmt::Comment(s) => self.fold_comment(s),
            Stmt::Label(s) => self.fold_label(s),
            Stmt::Inhale(expr, folding) => self.fold_inhale(expr, folding),
            Stmt::Exhale(e, p) => self.fold_exhale(e, p),
            Stmt::Assert(expr, folding, pos) => self.fold_assert(expr, folding, pos),
            Stmt::MethodCall(s, ve, vv) => self.fold_method_call(s, ve, vv),
            Stmt::Assign(p, e, k) => self.fold_assign(p, e, k),
            Stmt::Fold(s, ve, perm, variant, p) => self.fold_fold(s, ve, perm, variant, p),
            Stmt::Unfold(s, ve, perm, variant) => self.fold_unfold(s, ve, perm, variant),
            Stmt::Obtain(e, p) => self.fold_obtain(e, p),
            Stmt::WeakObtain(e) => self.fold_weak_obtain(e),
            Stmt::Havoc => self.fold_havoc(),
            Stmt::BeginFrame => self.fold_begin_frame(),
            Stmt::EndFrame => self.fold_end_frame(),
            Stmt::TransferPerm(a, b, c) => self.fold_transfer_perm(a, b, c),
            Stmt::PackageMagicWand(w, s, l, v, p) => self.fold_package_magic_wand(w, s, l, v, p),
            Stmt::ApplyMagicWand(w, p) => self.fold_apply_magic_wand(w, p),
            Stmt::ExpireBorrows(d) => self.fold_expire_borrows(d),
            Stmt::If(g, t) => self.fold_if(g, t),
        }
    }

    fn fold_expr(&mut self, e: Expr) -> Expr {
        e
    }

    fn fold_comment(&mut self, s: String) -> Stmt {
        Stmt::Comment(s)
    }

    fn fold_label(&mut self, s: String) -> Stmt {
        Stmt::Label(s)
    }

    fn fold_inhale(&mut self, expr: Expr, folding: FoldingBehaviour) -> Stmt {
        Stmt::Inhale(self.fold_expr(expr), folding)
    }

    fn fold_exhale(&mut self, e: Expr, p: Position) -> Stmt {
        Stmt::Exhale(self.fold_expr(e), p)
    }

    fn fold_assert(&mut self, expr: Expr, folding: FoldingBehaviour, pos: Position) -> Stmt {
        Stmt::Assert(self.fold_expr(expr), folding, pos)
    }

    fn fold_method_call(&mut self, s: String, ve: Vec<Expr>, vv: Vec<LocalVar>) -> Stmt {
        Stmt::MethodCall(s, ve.into_iter().map(|e| self.fold_expr(e)).collect(), vv)
    }

    fn fold_assign(&mut self, p: Expr, e: Expr, k: AssignKind) -> Stmt {
        Stmt::Assign(self.fold_expr(p), self.fold_expr(e), k)
    }

    fn fold_fold(
        &mut self,
        s: String,
        ve: Vec<Expr>,
        perm: PermAmount,
        variant: MaybeEnumVariantIndex,
        p: Position
    ) -> Stmt {
        Stmt::Fold(
            s,
            ve.into_iter().map(|e| self.fold_expr(e)).collect(),
            perm,
            variant,
            p,
        )
    }

    fn fold_unfold(
        &mut self,
        s: String,
        ve: Vec<Expr>,
        perm: PermAmount,
        variant: MaybeEnumVariantIndex,
    ) -> Stmt {
        Stmt::Unfold(s, ve.into_iter().map(|e| self.fold_expr(e)).collect(), perm, variant)
    }

    fn fold_obtain(&mut self, e: Expr, p: Position) -> Stmt {
        Stmt::Obtain(self.fold_expr(e), p)
    }

    fn fold_weak_obtain(&mut self, e: Expr) -> Stmt {
        Stmt::WeakObtain(self.fold_expr(e))
    }

    fn fold_havoc(&mut self) -> Stmt {
        Stmt::Havoc
    }

    fn fold_begin_frame(&mut self) -> Stmt {
        Stmt::BeginFrame
    }

    fn fold_end_frame(&mut self) -> Stmt {
        Stmt::EndFrame
    }

    fn fold_transfer_perm(&mut self, a: Expr, b: Expr, unchecked: bool) -> Stmt {
        Stmt::TransferPerm(self.fold_expr(a), self.fold_expr(b), unchecked)
    }

    fn fold_package_magic_wand(
        &mut self,
        wand: Expr,
        body: Vec<Stmt>,
        label: String,
        vars: Vec<LocalVar>,
        pos: Position,
    ) -> Stmt {
        Stmt::PackageMagicWand(
            self.fold_expr(wand),
            body.into_iter().map(|x| self.fold(x)).collect(),
            label,
            vars,
            pos,
        )
    }

    fn fold_apply_magic_wand(&mut self, w: Expr, p: Position) -> Stmt {
        Stmt::ApplyMagicWand(self.fold_expr(w), p)
    }

    fn fold_expire_borrows(&mut self, dag: ReborrowingDAG) -> Stmt {
        Stmt::ExpireBorrows(dag)
    }

    fn fold_if(&mut self, g: Expr, t: Vec<Stmt>) -> Stmt {
        Stmt::If(
            self.fold_expr(g),
            t.into_iter().map(|x| self.fold(x)).collect(),
        )
    }
}

pub trait StmtWalker {
    fn walk(&mut self, e: &Stmt) {
        match e {
            Stmt::Comment(s) => self.walk_comment(s),
            Stmt::Label(s) => self.walk_label(s),
            Stmt::Inhale(expr, folding) => self.walk_inhale(expr, folding),
            Stmt::Exhale(e, p) => self.walk_exhale(e, p),
            Stmt::Assert(expr, folding, pos) => self.walk_assert(expr, folding, pos),
            Stmt::MethodCall(s, ve, vv) => self.walk_method_call(s, ve, vv),
            Stmt::Assign(p, e, k) => self.walk_assign(p, e, k),
            Stmt::Fold(s, ve, perm, variant, pos) => self.walk_fold(s, ve, perm, variant, pos),
            Stmt::Unfold(s, ve, perm, variant) => self.walk_unfold(s, ve, perm, variant),
            Stmt::Obtain(e, p) => self.walk_obtain(e, p),
            Stmt::WeakObtain(e) => self.walk_weak_obtain(e),
            Stmt::Havoc => self.walk_havoc(),
            Stmt::BeginFrame => self.walk_begin_frame(),
            Stmt::EndFrame => self.walk_end_frame(),
            Stmt::TransferPerm(a, b, c) => self.walk_transfer_perm(a, b, c),
            Stmt::PackageMagicWand(w, s, l, v, p) => self.walk_package_magic_wand(w, s, l, v, p),
            Stmt::ApplyMagicWand(w, p) => self.walk_apply_magic_wand(w, p),
            Stmt::ExpireBorrows(d) => self.walk_expire_borrows(d),
            Stmt::If(g, t) => self.walk_if(g, t),
        }
    }

    fn walk_expr(&mut self, e: &Expr) {}

    fn walk_local_var(&mut self, local_var: &LocalVar) {}

    fn walk_comment(&mut self, s: &str) {}

    fn walk_label(&mut self, s: &str) {}

    fn walk_inhale(&mut self, expr: &Expr, folding: &FoldingBehaviour) {
        self.walk_expr(expr);
    }

    fn walk_exhale(&mut self, e: &Expr, p: &Position) {
        self.walk_expr(e);
    }

    fn walk_assert(&mut self, expr: &Expr, folding: &FoldingBehaviour, pos: &Position) {
        self.walk_expr(expr);
    }

    fn walk_method_call(&mut self, s: &str, ve: &Vec<Expr>, vv: &Vec<LocalVar>) {
        for a in ve {
            self.walk_expr(a);
        }
        for t in vv {
            self.walk_local_var(t);
        }
    }

    fn walk_assign(&mut self, p: &Expr, e: &Expr, k: &AssignKind) {
        self.walk_expr(p);
        self.walk_expr(e);
    }

    fn walk_fold(
        &mut self,
        s: &str,
        ve: &Vec<Expr>,
        perm: &PermAmount,
        variant: &MaybeEnumVariantIndex,
        p: &Position
    ) {
        for a in ve {
            self.walk_expr(a);
        }
    }

    fn walk_unfold(
        &mut self,
        s: &str,
        ve: &Vec<Expr>,
        perm: &PermAmount,
        variant: &MaybeEnumVariantIndex,
    ) {
        for a in ve {
            self.walk_expr(a);
        }
    }

    fn walk_obtain(&mut self, e: &Expr, _p: &Position) {
        self.walk_expr(e);
    }

    fn walk_weak_obtain(&mut self, e: &Expr) {
        self.walk_expr(e);
    }

    fn walk_havoc(&mut self) {}

    fn walk_begin_frame(&mut self) {}

    fn walk_end_frame(&mut self) {}

    fn walk_transfer_perm(&mut self, a: &Expr, b: &Expr, unchecked: &bool) {
        self.walk_expr(a);
        self.walk_expr(b);
    }

    fn walk_package_magic_wand(
        &mut self,
        wand: &Expr,
        body: &Vec<Stmt>,
        label: &str,
        vars: &[LocalVar],
        _p: &Position,
    ) {
        self.walk_expr(wand);
        for var in vars {
            self.walk_local_var(var);
        }
        for statement in body {
            self.walk(statement);
        }
    }

    fn walk_apply_magic_wand(&mut self, w: &Expr, _p: &Position) {
        self.walk_expr(w);
    }

    fn walk_expire_borrows(&mut self, dag: &ReborrowingDAG) {}

    fn walk_nested_cfg(&mut self, entry: &CfgBlockIndex, exit: &CfgBlockIndex) {}

    fn walk_if(&mut self, g: &Expr, t: &Vec<Stmt>) {
        self.walk_expr(g);
        for s in t {
            self.walk(s);
        }
    }
}

pub fn stmts_to_str(stmts: &[Stmt]) -> String {
    stmts
        .iter()
        .map(|stmt| format!("{}\n", stmt))
        .collect::<String>()
}
