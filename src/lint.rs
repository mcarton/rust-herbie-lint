use conf;
use itertools::Itertools;
use lisp::LispExpr;
use lisp;
use rusqlite as sql;
use rustc::front::map::Node;
use rustc::lint::{LateContext, LintArray, LintContext, LintPass, LateLintPass};
use rustc::middle::ty::TypeVariants;
use rustc_front::hir::*;
use std::borrow::Cow;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std;
use syntax::ast::MetaItemKind;
use syntax::ast::{Attribute, FloatTy};
use wait_timeout::ChildExt;

#[derive(Debug)]
pub struct Herbie {
    conf: Option<conf::Conf>,
    initialized: bool,
    subs: Vec<(LispExpr, LispExpr)>,
}

#[derive(Debug)]
pub enum InitError {
    Conf {
        error: conf::ConfError,
    },
    SQL {
        error: sql::Error,
    },
}

impl From<conf::ConfError> for InitError {
    fn from(err: conf::ConfError) -> InitError {
        InitError::Conf { error: err }
    }
}

impl From<sql::Error> for InitError {
    fn from(err: sql::Error) -> InitError {
        InitError::SQL { error: err }
    }
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            InitError::Conf { ref error } => write!(f, "Configuration error: {}", error),
            InitError::SQL { ref error } => write!(f, "Got SQL error: {}", error),
        }
    }
}

impl Herbie {
    pub fn new() -> Herbie {
        Herbie {
            conf: None,
            initialized: false,
            subs: Vec::new(),
        }
    }

    pub fn init(&mut self) -> Result<(), InitError> {
        if self.initialized {
            return Ok(())
        }

        self.initialized = true;

        let conf = try!(conf::read_conf());
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
        fn is_herbie_ignore(attr: &Attribute) -> bool {
            if let MetaItemKind::Word(ref word) = attr.node.value.node {
                word == &"herbie_ignore"
            }
            else {
                false
            }
        }

        let attrs = match cx.tcx.map.find(cx.tcx.map.get_parent(expr.id)) {
            Some(Node::NodeItem(item)) => &item.attrs,
            Some(Node::NodeTraitItem(item)) => &item.attrs,
            Some(Node::NodeImplItem(item)) => &item.attrs,
            _ => panic!("In herbie-lint: how did I get there?"),
        };

        if attrs.iter().any(is_herbie_ignore) {
            return;
        }

        let ty = cx.tcx.expr_ty(expr);

        if ty.sty != TypeVariants::TyFloat(FloatTy::F64) {
            return;
        }

        if let Err(err) =  self.init() {
            cx.span_lint_note(
                HERBIE,
                cx.krate.span,
                "Could not initialize Herbie-Lint",
                cx.krate.span,
                &err.to_string()
            );
            return;
        }

        let mut got_match = false;
        for &(ref cmdin, ref cmdout) in &self.subs {
            if let Some(bindings) = LispExpr::match_expr(expr, cmdin) {
                report(cx, expr, cmdout, &bindings);
                got_match = true;
            }
        }

        let conf = self.conf.as_ref().expect("Configuration should be read by now");
        if !got_match && conf.use_herbie != conf::UseHerbieConf::No {
            if let Err(err) = try_with_herbie(cx, expr, &conf) {
                cx.span_lint(HERBIE, expr.span, &err);
            }
        }
    }
}

fn try_with_herbie(cx: &LateContext, expr: &Expr, conf: &conf::Conf) -> Result<(), Cow<'static, str>> {
    let (lisp_expr, nb_ids, bindings) = match LispExpr::from_expr(expr) {
        Some(r) => r,
        // not an error, the expression might for example contain a function unknown to Herbie
        None => return Ok(()),
    };

    if lisp_expr.depth() <= 2 {
        return Ok(());
    }

    let seed: &str = &conf.herbie_seed;
    let mut command = Command::new("herbie-inout");
    let command = command
        .arg("--seed").arg(seed)
        .arg("-o").arg("rules:numerics")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
    ;

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return if conf.use_herbie == conf::UseHerbieConf::Yes {
                // TODO: wiki
                Err(format!("Could not call Herbie: {}", err).into())
            }
            else {
                Ok(())
            };
        }
    };

    // TODO: link to wiki about Herbie.toml
    cx.sess().diagnostic().span_note_without_error(
        expr.span,
        "Calling Herbie on the following expression, it might take a while"
    );

    let params = (0..nb_ids).map(|id| format!("herbie{}", id)).join(" ");
    let cmdin = lisp_expr.to_lisp("herbie");
    let lisp_expr = format!("(lambda ({}) {})\n", params, cmdin);
    let lisp_expr = lisp_expr.as_bytes();
    child.stdin
        .as_mut().expect("Herbie-inout's stdin not captured")
        .write(lisp_expr).expect("Could not write on herbie-inout's stdin")
    ;

    match conf.timeout {
        Some(timeout) => {
            match child.wait_timeout_ms(timeout*1000) {
                Ok(Some(status)) if status.success() => (),
                Ok(Some(status)) => {
                    return Err(format!("herbie-inout did not return successfully: status={}", status).into());
                }
                Ok(None) => {
                    cx.sess().diagnostic().span_note_without_error(expr.span, "Herbie timed out");
                    return Ok(());
                }
                Err(err) => {
                    return Err(format!("herbie-inout did not return successfully: {}", err).into());
                }
            }
        }
        None => {
            match child.wait() {
                Ok(status) if status.success() => (),
                Ok(status) => {
                    return Err(format!("herbie-inout did not return successfully: status={}", status).into());
                }
                Err(err) => {
                    return Err(format!("herbie-inout did not return successfully: {}", err).into());
                }
            }
        }
    }

    let mut stdout = if let Some(output) = child.stdout {
        output
    }
    else {
        return Err("cannot capture herbie-inout output".into());
    };

    let mut output = String::new();
    if let Err(err) = stdout.read_to_string(&mut output) {
        return Err(format!("cannot read output: {}", err).into());
    }

    let mut output = output.lines();

    let parse_error = |s: Option<&str>| -> Option<f64> {
        match s {
            Some(s) => {
                match s.split(' ').last().map(str::parse::<f64>) {
                    Some(Ok(f)) => Some(f),
                    _ => None,
                }
            }
            _ => None,
        }
    };

    let (errin, errout, cmdout) = match (parse_error(output.next()), parse_error(output.next()), output.next()) {
        (Some(errin), Some(errout), Some(cmdout)) => {
            (errin, errout, cmdout)
        }
        _ => {
            return Err("Could not parse herbie-inout output".into())
        }
    };


    if errin <= errout {
        return Ok(());
    }

    let mut parser = lisp::Parser::new();
    let cmdout = match parser.parse(cmdout) {
        Ok(cmdout) => cmdout,
        _ => return Err("Could not understand herbie-inout cmdout".into()),
    };

    report(cx, expr, &cmdout, &bindings);
    save(conf, &cmdin, &cmdout, "", errin, errout)
        .map_err(|err| format!("Could not save database, got SQL error {}", err).into())
}

fn report(cx: &LateContext, expr: &Expr, cmdout: &LispExpr, bindings: &lisp::MatchBindings) {
    cx.struct_span_lint(HERBIE, expr.span, "Numerically unstable expression")
      .span_suggestion(expr.span, "Try this", cmdout.to_rust(cx, &bindings))
      .emit();
}

fn save(
    conf: &conf::Conf,
    cmdin: &str, cmdout: &LispExpr,
    seed: &str,
    errin: f64, errout: f64
) -> Result<(), sql::Error> {
    let conn = try!(sql::Connection::open_with_flags(
        conf.db_path.as_ref(), sql::SQLITE_OPEN_READ_WRITE
    ));

    try!(conn.execute("INSERT INTO HerbieResults (cmdin, cmdout, opts, errin, errout)
                       VALUES ($1, $2, $3, $4, $5)",
                       &[&cmdin, &cmdout.to_lisp("herbie"), &seed, &errin, &errout]));

    Ok(())
}
