use conf;
use itertools::Itertools;
use lisp::LispExpr;
use lisp;
use rusqlite as sql;
use rustc::lint::{LateContext, LintArray, LintContext, LintPass, LateLintPass};
use rustc::middle::ty::TypeVariants;
use rustc_front::hir::*;
use std::io::Write;
use std::process::{Command, Stdio};
use std::str::from_utf8;
use syntax::ast::FloatTy;

#[derive(Debug)]
pub struct Herbie {
    conf: Option<conf::Conf>,
    initialized: bool,
    subs: Vec<(LispExpr, LispExpr)>,
}

impl Herbie {

    pub fn new() -> Herbie {
        Herbie {
            conf: None,
            initialized: false,
            subs: Vec::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), sql::Error> {
        if self.initialized {
            return Ok(())
        }

        self.initialized = true;

        let conf = conf::read_conf();
        let conn = try!(sql::Connection::open_with_flags(
            conf.db_path.as_ref(), sql::SQLITE_OPEN_READ_ONLY
        ));
        let mut query = try!(conn.prepare("SELECT * FROM HerbieResults"));

        self.subs = try!(query.query(&[])).filter_map(|row| {
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
        }).collect();

        self.conf = Some(conf);

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

        if let Err(err) =  self.init() {
            cx.span_lint_note(
                HERBIE,
                cx.krate.span,
                "Could not initialize Herbie-Lint",
                cx.krate.span,
                &format!("Got SQL error: {}", err)
            );
        }

        let mut got_match = false;
        for &(ref cmdin, ref cmdout) in &self.subs {
            if let Some(bindings) = LispExpr::match_expr(expr, cmdin) {
                report(cx, expr, cmdout, &bindings);
                got_match = true;
            }
        }

        let conf = self.conf.as_ref().unwrap();
        if !got_match && conf.use_herbie != conf::UseHerbieConf::No {
            try_with_herbie(cx, expr, &conf.herbie_seed);
        }
    }
}

fn try_with_herbie(cx: &LateContext, expr: &Expr, seed: &str) {
    let (lisp_expr, nb_ids, bindings) = match LispExpr::from_expr(expr) {
        Some(r) => r,
        None => return, // TODO: report
    };

    if lisp_expr.depth() <= 2 {
        return;
    }

    // TODO: link to wiki about Herbie.toml
    cx.sess().diagnostic().span_note_without_error(expr.span, "Calling Herbie on the following expression, it might take a while");

    let mut child = Command::new("herbie-inout")
        .arg("--seed").arg(seed)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let params = (0..nb_ids).map(|id| format!("herbie{}", id)).join(" ");
    let lisp_expr = lisp_expr.to_lisp("herbie");
    let lisp_expr = format!("(lambda ({}) {})\n", params, lisp_expr);
    let lisp_expr = lisp_expr.as_bytes();
    child.stdin.as_mut().unwrap().write(lisp_expr).unwrap();

    let output = if let Ok(output) = child.wait_with_output() {
        if output.status.success() {
            if let Ok(output) = from_utf8(&output.stdout) {
                output.to_owned()
            }
            else {
                return
            }
        }
        else {
            return
        }
    }
    else {
        return
    };

    let mut output = output.lines();
    let errin = output.next().unwrap().split(' ').last().unwrap().parse::<f64>().unwrap();
    let errout = output.next().unwrap().split(' ').last().unwrap().parse::<f64>().unwrap();

    if errin <= errout {
        return
    }

    let mut parser = lisp::Parser::new();
    let cmdout = parser.parse(&output.next().unwrap()).unwrap();

    report(cx, expr, &cmdout, &bindings);
}

fn report(cx: &LateContext, expr: &Expr, cmdout: &LispExpr, bindings: &lisp::MatchBindings) {
    cx.struct_span_lint(HERBIE, expr.span, "Numerically unstable expression")
      .span_suggestion(expr.span, "Try this", cmdout.to_rust(cx, &bindings))
      .emit();
}
