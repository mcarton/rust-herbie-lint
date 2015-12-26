use lisp::LispExpr;
use lisp;
use rusqlite as sql;
use rustc::lint::{LateContext, LintArray, LintContext, LintPass, LateLintPass};
use rustc::middle::ty::TypeVariants;
use rustc_front::hir::*;
use syntax::ast::FloatTy;

#[derive(Debug)]
pub struct Herbie {
    pub subs: Option<Vec<(LispExpr, LispExpr)>>,
}

impl Herbie {

    pub fn new() -> Herbie {
        Herbie {
            subs: None,
        }
    }

    pub fn init(&mut self) -> Result<(), sql::Error> {
        let conn = try!(sql::Connection::open_with_flags("Herbie.db", sql::SQLITE_OPEN_READ_ONLY));
        let mut query = try!(conn.prepare("SELECT * FROM HerbieResults"));

        self.subs = Some(try!(query.query(&[])).filter_map(|row| {
            match row {
                Ok(row) => {
                    let cmdin : String = row.get(1);
                    let cmdout : String = row.get(2);
                    // row.get(3) is opts ↔ Herbies options
                    let errin = row.get_checked(4).unwrap_or(0.);
                    let errout = row.get_checked(5).unwrap_or(0.);

                    if cmdin == cmdout || errin <= errout {
                        return None;
                    }

                    let mut parser = lisp::Parser::new();
                    match parser.parse(&cmdin) {
                        Ok(cmdin) => {
                            match parser.parse(&cmdout) {
                                Ok(cmdout) => {
                                    Some((cmdin, cmdout))
                                }
                                Err(..) => None,
                            }
                        }
                        Err(..) => None,
                    }
                }
                Err(..) => None,
            }
        }).collect());

        Ok(())
    }

}

declare_lint!(pub HERBIE, Warn,
              "checks for numerical instability");

impl LintPass for Herbie {
    fn get_lints(&self) -> LintArray {
        lint_array!(HERBIE)
    }
}

impl LateLintPass for Herbie {
    fn check_expr(&mut self, cx: &LateContext, expr: &Expr) {
        let ty = cx.tcx.expr_ty(expr);

        if ty.sty != TypeVariants::TyFloat(FloatTy::TyF64) {
            return;
        }

        if self.subs.is_none() {
            self.init().expect("Could not initialize Herbie-Lint");
        }

        for &(ref cmdin, ref cmdout) in self.subs.as_ref().unwrap() {
            if let Some(bindings) = LispExpr::match_expr(expr, cmdin) {
                cx.span_lint(HERBIE, expr.span, "Numerically unstable expression");
                cx.sess().span_suggestion(expr.span, "Try this", cmdout.to_rust(cx, &bindings));
            }
        }
    }
}
