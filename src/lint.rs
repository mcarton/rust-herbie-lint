use lisp::LispExpr;
use lisp;
use rusqlite as sql;
use rustc::lint::{LateContext, LintArray, LintContext, LintPass, LateLintPass};
use rustc_front::hir::*;

#[derive(Debug)]
pub struct Herbie {
    pub subs: Vec<(LispExpr, LispExpr)>,
}

impl Herbie {

    pub fn new() -> Result<Herbie, sql::Error> {
        let conn = try!(sql::Connection::open_with_flags("Herbie.db", sql::SQLITE_OPEN_READ_ONLY));
        let mut query = try!(conn.prepare("SELECT * FROM HerbieResults"));
        Ok(Herbie {
            subs: try!(query.query(&[])).filter_map(|row| {
                match row {
                    Ok(row) => {
                        let cmdin : String = row.get(1);
                        let cmdout : String = row.get(2);

                        match lisp::parse(&cmdin) {
                            Ok(cmdin) => {
                                match lisp::parse(&cmdout) {
                                    Ok(cmdout) => {
                                        if !cmdin.is_form_of(&cmdout) {
                                            Some((cmdin, cmdout))
                                        }
                                        else {
                                            None
                                        }
                                    }
                                    Err(..) => None,
                                }
                            }
                            Err(..) => None,
                        }
                    }
                    Err(..) => None,
                }
            }).collect(),
        })
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
        if let Ok(lisp) = LispExpr::from_expr(expr) {
            if let Some(&(_, ref cmdout)) = self.subs.iter().find(|&&(ref cmdin, _)| lisp.is_form_of(cmdin)) {
                cx.span_lint(HERBIE, expr.span, "Numerically unstable expression");
                // TODO: rustify
                cx.sess().span_suggestion(expr.span, "Try this", cmdout.to_lisp());
            }
        }
    }
}
