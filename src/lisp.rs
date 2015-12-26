#![cfg_attr(feature="clippy", allow(float_cmp))]

use rustc::lint::LateContext;
use rustc_front::hir::*;
use rustc_front::util::{binop_to_string, unop_to_string};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std;
use syntax::ast::Lit_::*;
use syntax::ast::{FloatTy, Name};
use syntax::codemap::{Span, Spanned};
use utils::{merge_span, snippet};

pub enum LispExpr {
    Binary(BinOp_, Box<LispExpr>, Box<LispExpr>),
    Fun(String, Vec<LispExpr>),
    Ident(u64),
    Lit(f64),
    Unary(UnOp, Box<LispExpr>),
}

impl std::fmt::Debug for LispExpr {

    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.pad(&self.to_lisp())
    }

}

/// List (herbie_name, rust_name).
/// Warning: *MUST* be alphabetized on Herbie name.
/// Herbie also supports the following:
///   * cot (cotangent),
///   * expt (expi, expf),
///   * mod
///   * sqt (square),
const KNOW_FUNS : &'static [(&'static str, &'static str, usize)] = &[
    ("abs",   "abs",    1),
    ("acos",  "acos",   1),
    ("asin",  "asin",   1),
    ("atan",  "atan",   1),
    ("atan2", "atan2",  2),
    ("cos",   "cos",    1),
    ("cosh",  "cosh",   1),
    ("exp",   "exp",    1),
    ("expm1", "exp_m1", 1),
    ("hypot", "hypot",  2),
    ("log",   "ln",     1),
    ("log1p", "ln_1p",  1),
    ("sin",   "sin",    1),
    ("sinh",  "sinh",   1),
    ("sqrt",  "sqrt",   1),
    ("tan",   "tan",    1),
    ("tanh",  "tanh",   1),
];

enum MatchBinding {
    Field(Option<QSelf>, Path, Spanned<Name>),
    Ident(Option<QSelf>, Path),
    Lit(f64, Span),
    Other(Span),
    TupField(Option<QSelf>, Path, Spanned<usize>),
}

pub struct MatchBindings {
    bindings: HashMap<u64, MatchBinding>
}

impl LispExpr {

    pub fn match_expr(matchee: &Expr, other: &LispExpr) -> Option<MatchBindings> {

        fn match_expr_impl(lhs: &Expr, rhs: &LispExpr, ids: &mut HashMap<u64, MatchBinding>) -> bool {
            fn bind_unknown(rid: u64, span: Span, ids: &mut HashMap<u64, MatchBinding>) -> bool {
                if let Entry::Vacant(vacant) = ids.entry(rid) {
                    vacant.insert(MatchBinding::Other(span));
                    true
                }
                else {
                    false
                }
            }

            match (&lhs.node, rhs) {
                (&ExprBinary(lop, ref lp1, ref lp2), &LispExpr::Binary(rop, ref rp1, ref rp2)) => {
                    lop.node == rop
                    && match_expr_impl(lp1, rp1, ids)
                    && match_expr_impl(lp2, rp2, ids)
                },
                (&ExprMethodCall(ref lfun, ref ascripted_type, ref lp), &LispExpr::Fun(ref rfun, ref rp)) if ascripted_type.is_empty() => {
                    let name = lfun.node.as_str();
                    if let Some(&(herbie_name, _, _)) = KNOW_FUNS.iter().find(
                        |&&(_, rust_name, num_params)| {
                            rust_name == name && lp.len() == num_params
                        }
                    ) {
                        herbie_name == rfun
                        && lp.iter().zip(rp).all(|(lp, rp)| match_expr_impl(lp, rp, ids))
                    }
                    else {
                        false
                    }
                },
                (&ExprPath(ref qualif, ref path), &LispExpr::Ident(rid)) => {
                    match ids.entry(rid) {
                        Entry::Occupied(entry) => {
                            if let MatchBinding::Ident(ref bqualif, ref bpath) = *entry.get() {
                                qualif == bqualif
                                && path.global == bpath.global
                                && &path.segments == &bpath.segments
                            }
                            else {
                                false
                            }
                        },
                        Entry::Vacant(vacant) => {
                            vacant.insert(MatchBinding::Ident(qualif.clone(), path.clone()));
                            true
                        }
                    }
                },
                (&ExprLit(ref lit), &LispExpr::Lit(r)) => {
                    match lit.node {
                        LitFloat(ref f, FloatTy::TyF64) | LitFloatUnsuffixed(ref f) => {
                            f.parse() == Ok(r)
                        },
                        _ => false
                    }
                },
                (&ExprLit(ref expr), &LispExpr::Ident(rid)) => {
                    match expr.node {
                        LitFloat(ref lit, FloatTy::TyF64) | LitFloatUnsuffixed(ref lit) => {
                            if let Ok(lit) = lit.parse() {
                                match ids.entry(rid) {
                                    Entry::Occupied(entry) => {
                                        if let MatchBinding::Lit(binded, _) = *entry.get() {
                                            lit == binded
                                        }
                                        else {
                                            false
                                        }
                                    },
                                    Entry::Vacant(vacant) => {
                                        vacant.insert(MatchBinding::Lit(lit, expr.span));
                                        true
                                    }
                                }
                            }
                            else {
                                bind_unknown(rid, lhs.span, ids)
                            }
                        },
                        _ => bind_unknown(rid, lhs.span, ids)
                    }
                },
                (&ExprUnary(lop, ref lp), &LispExpr::Unary(rop, ref rp)) => {
                    lop == rop && match_expr_impl(lp, rp, ids)
                },
                (&ExprTupField(ref tup, ref idx), &LispExpr::Ident(rid)) => {
                    if let ExprPath(ref qualif, ref path) = tup.node {
                        return match ids.entry(rid) {
                            Entry::Occupied(entry) => {
                                if let MatchBinding::TupField(ref bqualif, ref bpath, bidx) = *entry.get() {
                                    qualif == bqualif
                                    && path.global == bpath.global
                                    && path.segments == bpath.segments
                                    && idx.node == bidx.node
                                }
                                else {
                                    false
                                }
                            },
                            Entry::Vacant(vacant) => {
                                vacant.insert(MatchBinding::TupField(qualif.clone(), path.clone(), idx.clone()));
                                true
                            }
                        }
                    }

                    bind_unknown(rid, lhs.span, ids)
                },
                (&ExprField(ref expr, ref name), &LispExpr::Ident(rid)) => {
                    if let ExprPath(ref qualif, ref path) = expr.node {
                        return match ids.entry(rid) {
                            Entry::Occupied(entry) => {
                                if let MatchBinding::Field(ref bqualif, ref bpath, ref bname) = *entry.get() {
                                    qualif == bqualif
                                    && path.global == bpath.global
                                    && path.segments == bpath.segments
                                    && name.node == bname.node
                                }
                                else {
                                    false
                                }
                            },
                            Entry::Vacant(vacant) => {
                                vacant.insert(MatchBinding::Field(qualif.clone(), path.clone(), name.clone()));
                                true
                            }
                        }
                    }

                    bind_unknown(rid, lhs.span, ids)
                },
                (_, &LispExpr::Ident(rid)) => {
                    bind_unknown(rid, lhs.span, ids)
                },
                _ => false,
            }
        }

        let mut ids = HashMap::new();
        if match_expr_impl(matchee, other, &mut ids) {
            Some(MatchBindings { bindings: ids })
        }
        else {
            None
        }
    }

    // TODO: should probably not be pub
    pub fn to_lisp(&self) -> String {
        match *self {
            LispExpr::Binary(op, ref lhs, ref rhs) => {
                format!("({} {} {})", binop_to_string(op), lhs.to_lisp(), rhs.to_lisp())
            },
            LispExpr::Fun(ref name, ref params) => {
                let mut buf = String::new();
                buf.push('(');
                buf.push_str(name);

                for p in params {
                    buf.push(' ');
                    buf.push_str(&p.to_lisp());
                }

                buf.push(')');
                buf
            },
            LispExpr::Lit(f) => {
                format!("{}", f)
            },
            LispExpr::Unary(op, ref expr) => {
                format!("({} {})", unop_to_string(op), expr.to_lisp())
            },
            LispExpr::Ident(id) => {
                format!("${}", id)
            },
        }
    }

    pub fn to_rust(&self, cx: &LateContext, bindings: &MatchBindings) -> String {
        fn to_rust_impl(expr: &LispExpr, cx: &LateContext, bindings: &MatchBindings) -> (String, bool) {
            match *expr {
                LispExpr::Binary(op, ref lhs, ref rhs) => {
                    match (to_rust_impl(lhs, cx, bindings), to_rust_impl(rhs, cx, bindings)) {
                        ((lhs, false), (rhs, false)) => {
                            (format!("{} {} {}", lhs, binop_to_string(op), rhs), true)
                        },
                        ((lhs, true), (rhs, false)) => {
                            (format!("({}) {} {}", lhs, binop_to_string(op), rhs), true)
                        },
                        ((lhs, false), (rhs, true)) => {
                            (format!("{} {} ({})", lhs, binop_to_string(op), rhs), true)
                        },
                        ((lhs, true), (rhs, true)) => {
                            (format!("({}) {} ({})", lhs, binop_to_string(op), rhs), true)
                        },
                    }
                },
                LispExpr::Fun(ref name, ref params) => {
                    let mut buf = String::new();
                    match to_rust_impl(&params[0], cx, bindings) {
                        (expr, false) => buf.push_str(&expr),
                        (expr, true) => {
                            buf.push('(');
                            buf.push_str(&expr);
                            buf.push(')');
                        },
                    }
                    buf.push('.');
                    buf.push_str(name);
                    buf.push('(');

                    for (i, p) in params.iter().skip(1).enumerate() {
                        if i != 0 {
                            buf.push_str(", ");
                        }
                        buf.push_str(&to_rust_impl(p, cx, bindings).0);
                    }

                    buf.push(')');
                    (buf, false)
                },
                LispExpr::Lit(f) => {
                    (format!("{}", f), false)
                },
                LispExpr::Unary(op, ref expr) => {
                    match to_rust_impl(expr, cx, bindings) {
                        (expr, false) => (format!("{}{}", unop_to_string(op), expr), true),
                        (expr, true) => (format!("{}({})", unop_to_string(op), expr), true),
                    }
                },
                LispExpr::Ident(id) => {
                    match *bindings.bindings.get(&id).expect("Got an unbinded id!") {
                        MatchBinding::Field(_, ref path, ref name) => {
                            (snippet(cx, merge_span(path.span, name.span), "..").into_owned(), false)
                        },
                        MatchBinding::Ident(_, ref path) => {
                            (snippet(cx, path.span, "..").into_owned(), false)
                        },
                        MatchBinding::Lit(_, ref span) => {
                            (snippet(cx, *span, "..").into_owned(), false)
                        },
                        MatchBinding::Other(ref span) => {
                            (snippet(cx, *span, "..").into_owned(), true)
                        },
                        MatchBinding::TupField(_, ref path, ref idx) => {
                            (snippet(cx, merge_span(path.span, idx.span), "..").into_owned(), false)
                        },
                    }
                },
            }
        }

        to_rust_impl(self, cx, bindings).0
    }

}

#[derive(Debug)]
pub enum ParseError {
    Arity,
    Expected(char),
    Ident,
    Float,
    Unexpected(char),
    EOE,
}

struct Parser {
    stack: Vec<char>,
}

pub fn parse(s: &str) -> Result<LispExpr, ParseError> {
    let mut parser = Parser { stack: Vec::new() };
    let mut it = s.chars();

    match parser.parse_impl(&mut it) {
        Ok(result) => {
            if it.next().is_some() {
                Err(ParseError::EOE)
            }
            else  {
                Ok(result)
            }
        },
        err @ Err(..) => err,
    }
}

impl Parser {

    fn parse_impl<It: Iterator<Item=char>>(&mut self, it: &mut It) -> Result<LispExpr, ParseError> {
        match self.get_char(it, true) {
            Some('(') => {
                match self.get_char(it, true) {
                    Some('+') => self.parse_op(it, BinOp_::BiAdd),
                    Some('-') => self.parse_op(it, BinOp_::BiSub),
                    Some('*') => self.parse_op(it, BinOp_::BiMul),
                    Some('/') => self.parse_op(it, BinOp_::BiDiv),
                    Some(c) => {
                        self.put_back(c);
                        self.parse_fun(it)
                    },
                    None => Err(ParseError::EOE),
                }
            },
            Some(c) if c.is_digit(10) => {
                self.put_back(c);
                self.parse_float(it)
            },
            Some('h') => self.parse_ident(it),
            Some(c) => {
                self.put_back(c);
                Err(ParseError::Unexpected(c))
            }
            None => Err(ParseError::EOE),
        }
    }

    fn expect<It: Iterator<Item=char>>(&mut self, it: &mut It, c: char, skip_whitespace: bool) -> Result<(), ParseError> {
        if self.get_char(it, skip_whitespace) == Some(c) {
            Ok(())
        }
        else {
            Err(ParseError::Expected(c))
        }
    }

    fn parse_float<It: Iterator<Item=char>>(&mut self, it: &mut It) -> Result<LispExpr, ParseError> {
        let mut buf = String::new();
        loop {
            let c = self.get_char(it, false);
            if let Some(c) = c {
                if c.is_digit(10) || ['.', 'e', '+', '-'].contains(&c) {
                    buf.push(c);
                    continue;
                }

                self.put_back(c);
            }

            break;
        }

        match buf.parse() {
            Ok(f) => Ok(LispExpr::Lit(f)),
            Err(..) => Err(ParseError::Float),
        }
    }

    fn parse_ident<It: Iterator<Item=char>>(&mut self, it: &mut It) -> Result<LispExpr, ParseError> {
        // TODO: Herbie also supports ‘pi’ and ‘e’ as native constants.
        let mut buf = String::new();
        loop {
            let c = self.get_char(it, false);
            if let Some(c) = c {
                if buf.is_empty() && 'a' <= c && c <= 'z' {
                    continue;
                }
                else if c.is_digit(10)  {
                    buf.push(c);
                    continue;
                }
                else {
                    self.put_back(c);
                }
            }

            break;
        }

        match buf.parse() {
            Ok(id) => Ok(LispExpr::Ident(id)),
            Err(..) => Err(ParseError::Ident),
        }
    }

    fn parse_fun<It: Iterator<Item=char>>(&mut self, it: &mut It) -> Result<LispExpr, ParseError> {
        let mut buf = String::new();
        loop {
            let c = self.get_char(it, false);
            if let Some(c) = c {
                if 'a' <= c && c <= 'z' {
                    buf.push(c);
                    continue;
                }
                else {
                    self.put_back(c);
                }
            }

            break;
        }

        if !buf.is_empty() {
            let mut params = vec![];

            while let Ok(param) = self.parse_impl(it) {
                params.push(param);
            }

            try!(self.expect(it, ')', true));
            Ok(LispExpr::Fun(buf, params))
        }
        else {
            Err(ParseError::Ident)
        }
    }

    fn parse_op<It: Iterator<Item=char>>(&mut self, it: &mut It, op: BinOp_) -> Result<LispExpr, ParseError> {
        // TODO: Herbie seems to also support the following for the repip of a float: (/ 42) and
        // rust has a function recip for that
        let lhs = try!(self.parse_impl(it));
        let r = if let Ok(rhs) = self.parse_impl(it) {
            Ok(LispExpr::Binary(op, box lhs, box rhs))
        }
        else if op == BinOp_::BiSub {
            Ok(LispExpr::Unary(UnOp::UnNeg, box lhs))
        }
        else {
            return Err(ParseError::Arity);
        };
        try!(self.expect(it, ')', true));
        r
    }

    fn get_char<It: Iterator<Item=char>>(&mut self, it: &mut It, skip_whitespace: bool) -> Option<char> {
        loop {
            match self.stack.pop() {
                Some(e) if skip_whitespace && e.is_whitespace() => continue,
                Some(e) => return Some(e),
                None => break,
            }
        }

        loop {
            match it.next() {
                Some(e) if skip_whitespace && e.is_whitespace() => continue,
                Some(e) => return Some(e),
                None => return None,
            }
        }
    }

    fn put_back(&mut self, c: char) {
        self.stack.push(c);
    }
}
