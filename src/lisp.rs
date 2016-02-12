#![cfg_attr(feature="clippy", allow(float_cmp))]

use rustc::lint::LateContext;
use rustc_front::hir::*;
use rustc_front::util::{binop_to_string, unop_to_string};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::iter::FromIterator;
use std;
use syntax::ast::LitKind;
use syntax::ast::{FloatTy, Name};
use syntax::codemap::{Span, Spanned};
use utils::{merge_span, snippet};

#[derive(Clone, PartialEq)]
pub enum LispExpr {
    Binary(BinOp_, Box<LispExpr>, Box<LispExpr>),
    Fun(String, Vec<LispExpr>),
    Ident(u64),
    Lit(f64),
    Unary(UnOp, Box<LispExpr>),
}

impl std::fmt::Debug for LispExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.pad(&self.to_lisp("$"))
    }
}

/// List (herbie_name, rust_name).
/// Warning: *MUST* be alphabetized on Herbie name.
/// Herbie also supports the following:
///   * cot (cotangent),
///   * expt (Rust has powi vs. powf),
///   * mod,
///   * sqr (square),
const KNOWN_FUNS : &'static [(&'static str, &'static str, usize)] = &[
    ("abs",   "abs",    1),
    ("acos",  "acos",   1),
    ("asin",  "asin",   1),
    ("atan",  "atan",   1),
    ("atan2", "atan2",  2),
    ("cos",   "cos",    1),
    ("cosh",  "cosh",   1),
    ("exp",   "exp",    1),
    ("expm1", "exp_m1", 1),
    ("expt",  "powf",   2),
    ("hypot", "hypot",  2),
    ("log",   "ln",     1),
    ("log1p", "ln_1p",  1),
    ("sin",   "sin",    1),
    ("sinh",  "sinh",   1),
    ("sqrt",  "sqrt",   1),
    ("tan",   "tan",    1),
    ("tanh",  "tanh",   1),
];

fn rust_name(herbie_name: &str) -> Option<&'static str> {
    KNOWN_FUNS .iter()
               .find(|&&(name, _, _)| herbie_name == name)
               .map(|&(_, rust_name, _)| rust_name)
}

fn herbie_name(rust_name: &str, nb_params: usize) -> Option<&'static str> {
    KNOWN_FUNS.iter()
              .find(|&&(_, name, params)| rust_name == name && nb_params == params)
              .map(|t| t.0)
}

#[derive(Debug)]
enum MatchBinding {
    Field(Option<QSelf>, Path, Spanned<Name>),
    Ident(Option<QSelf>, Path),
    Lit(f64, Span),
    Other(Span),
    TupField(Option<QSelf>, Path, Spanned<usize>),
}

#[derive(Debug)]
pub struct MatchBindings {
    bindings: HashMap<u64, MatchBinding>,
}

impl LispExpr {
    pub fn from_expr(expr: &Expr) -> Option<(LispExpr, u64, MatchBindings)> {
        fn push_new_binding(
            binding: MatchBinding,
            ids: &mut Vec<MatchBinding>,
            curr_id: &mut u64
        ) -> Option<LispExpr> {
            ids.push(binding);
            let id = *curr_id;
            *curr_id += 1;
            Some(LispExpr::Ident(id))
        }

        fn from_expr_impl(
            expr: &Expr,
            ids: &mut Vec<MatchBinding>,
            curr_id: &mut u64
        ) -> Option<LispExpr> {
            match expr.node {
                ExprBinary(op, ref lhs, ref rhs) => {
                    if let Some(lhs_expr) = from_expr_impl(lhs, ids, curr_id) {
                        if let Some(rhs_expr) = from_expr_impl(rhs, ids, curr_id) {
                            return Some(LispExpr::Binary(op.node, box lhs_expr, box rhs_expr));
                        }
                    }

                    None
                }
                ExprField(ref expr, ref name) => {
                    if let ExprPath(ref qualif, ref path) = expr.node {
                        if let Some(pos) = ids.iter().position(|item| {
                            if let MatchBinding::Field(ref bqualif, ref bpath, ref bname) = *item {
                                bqualif == qualif
                                && bpath.global == path.global
                                && bpath.segments == path.segments
                                && bname.node == name.node
                            }
                            else {
                                false
                            }
                        }) {
                            Some(LispExpr::Ident(pos as u64))
                        }
                        else {
                            push_new_binding(MatchBinding::Field(qualif.clone(), path.clone(), *name), ids, curr_id)
                        }
                    }
                    else {
                        push_new_binding(MatchBinding::Other(expr.span), ids, curr_id)
                    }
                }
                ExprLit(ref lit) => {
                    match lit.node {
                        LitKind::Float(ref f, FloatTy::F64)
                        | LitKind::FloatUnsuffixed(ref f) => {
                            f.parse().ok().map(LispExpr::Lit)
                        }
                        _ => None,
                    }
                }
                ExprMethodCall(ref fun, ref ascripted_type, ref params) if ascripted_type.is_empty() => {
                    let name = fun.node.as_str();

                    if let Some(herbie_name) = herbie_name(&name, params.len()) {
                        let mut lisp_params = Vec::new();
                        for param in params {
                            if let Some(lisp_expr) = from_expr_impl(param, ids, curr_id) {
                                lisp_params.push(lisp_expr);
                            }
                            else {
                                return None;
                            }
                        }
                        Some(LispExpr::Fun(herbie_name.into(), lisp_params))
                    }
                    else {
                        None
                    }
                }
                ExprPath(ref qualif, ref path) => {
                    if let Some(pos) = ids.iter().position(|item| {
                        if let MatchBinding::Ident(ref bqualif, ref bpath) = *item {
                            bqualif == qualif
                            && bpath.global == path.global
                            && bpath.segments == path.segments
                        }
                        else {
                            false
                        }
                    }) {
                        Some(LispExpr::Ident(pos as u64))
                    }
                    else {
                        push_new_binding(MatchBinding::Ident(qualif.clone(), path.clone()), ids, curr_id)
                    }
                },
                ExprTupField(ref tup, ref idx) => {
                    if let ExprPath(ref qualif, ref path) = tup.node {
                        if let Some(pos) = ids.iter().position(|item| {
                            if let MatchBinding::TupField(ref bqualif, ref bpath, ref bidx) = *item {
                                bqualif == qualif
                                && bpath.global == path.global
                                && bpath.segments == path.segments
                                && bidx.node == idx.node
                            }
                            else {
                                false
                            }
                        }) {
                            Some(LispExpr::Ident(pos as u64))
                        }
                        else {
                            push_new_binding(MatchBinding::TupField(qualif.clone(), path.clone(), *idx), ids, curr_id)
                        }
                    }
                    else {
                        push_new_binding(MatchBinding::Other(expr.span), ids, curr_id)
                    }
                }
                ExprUnary(op, ref expr) => {
                    from_expr_impl(expr, ids, curr_id).map(|expr| LispExpr::Unary(op, box expr))
                }
                _ => None,
            }
        }

        let mut ids = Vec::new();
        let mut curr_id = 0;
        from_expr_impl(expr, &mut ids, &mut curr_id).map(|expr| {
            let bindings = ids.drain(..).enumerate().map(|(k, v)| (k as u64, v));
            (expr, curr_id, MatchBindings { bindings: HashMap::from_iter(bindings) })
        })
    }

    pub fn match_expr(matchee: &Expr, other: &LispExpr) -> Option<MatchBindings> {

        fn match_expr_impl(
            lhs: &Expr,
            rhs: &LispExpr,
            ids: &mut HashMap<u64, MatchBinding>
        ) -> bool {
            fn bind_unknown(rid: u64, span: Span, ids: &mut HashMap<u64, MatchBinding>) -> bool {
                if let Entry::Vacant(vacant) = ids.entry(rid) {
                    vacant.insert(MatchBinding::Other(span));
                    true
                }
                else {
                    false
                }
            }

            fn try_insert<Occ, Vac>(rid: u64, ids: &mut HashMap<u64, MatchBinding>, occ: Occ, vac: Vac) -> bool
            where Occ: FnOnce(&MatchBinding) -> bool,
                  Vac: FnOnce() -> MatchBinding {
                match ids.entry(rid) {
                    Entry::Occupied(entry) => occ(entry.get()),
                    Entry::Vacant(vacant) => {
                        vacant.insert(vac());
                        true
                    }
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
                    if let Some(herbie_name) = herbie_name(&name, lp.len()) {
                        herbie_name == rfun
                        && lp.iter().zip(rp).all(|(lp, rp)| match_expr_impl(lp, rp, ids))
                    }
                    else {
                        false
                    }
                }
                (&ExprPath(ref qualif, ref path), &LispExpr::Ident(rid)) => {
                    try_insert(rid, ids, |entry| {
                        if let MatchBinding::Ident(ref bqualif, ref bpath) = *entry {
                            qualif == bqualif
                            && path.global == bpath.global
                            && &path.segments == &bpath.segments
                        }
                        else {
                            false
                        }
                    }, || {
                        MatchBinding::Ident(qualif.clone(), path.clone())
                    })
                }
                (&ExprLit(ref lit), &LispExpr::Lit(r)) => {
                    match lit.node {
                        LitKind::Float(ref f, FloatTy::F64)
                        | LitKind::FloatUnsuffixed(ref f) => {
                            f.parse() == Ok(r)
                        }
                        _ => false,
                    }
                }
                (&ExprLit(ref expr), &LispExpr::Ident(rid)) => {
                    match expr.node {
                        LitKind::Float(ref lit, FloatTy::F64)
                        | LitKind::FloatUnsuffixed(ref lit) => {
                            if let Ok(lit) = lit.parse() {
                                try_insert(rid, ids, |entry| {
                                    if let MatchBinding::Lit(binded, _) = *entry {
                                        lit == binded
                                    }
                                    else {
                                        false
                                    }
                                }, || {
                                    MatchBinding::Lit(lit, expr.span)
                                })
                            }
                            else {
                                bind_unknown(rid, lhs.span, ids)
                            }
                        }
                        _ => bind_unknown(rid, lhs.span, ids),
                    }
                }
                (&ExprUnary(lop, ref lp), &LispExpr::Unary(rop, ref rp)) => {
                    lop == rop && match_expr_impl(lp, rp, ids)
                }
                (&ExprTupField(ref tup, ref idx), &LispExpr::Ident(rid)) => {
                    if let ExprPath(ref qualif, ref path) = tup.node {
                        return try_insert(rid, ids, |entry| {
                            if let MatchBinding::TupField(ref bqualif, ref bpath, bidx) = *entry {
                                qualif == bqualif
                                && path.global == bpath.global
                                && path.segments == bpath.segments
                                && idx.node == bidx.node
                            }
                            else {
                                false
                            }
                        }, || {
                            MatchBinding::TupField(qualif.clone(), path.clone(), *idx)
                        })
                    }

                    bind_unknown(rid, lhs.span, ids)
                }
                (&ExprField(ref expr, ref name), &LispExpr::Ident(rid)) => {
                    if let ExprPath(ref qualif, ref path) = expr.node {
                        return try_insert(rid, ids, |entry| {
                            if let MatchBinding::Field(ref bqualif, ref bpath, ref bname) = *entry {
                                qualif == bqualif
                                && path.global == bpath.global
                                && path.segments == bpath.segments
                                && name.node == bname.node
                            }
                            else {
                                false
                            }
                        }, || {
                            MatchBinding::Field(qualif.clone(), path.clone(), *name)
                        })
                    }

                    bind_unknown(rid, lhs.span, ids)
                }
                (_, &LispExpr::Ident(rid)) => bind_unknown(rid, lhs.span, ids),
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

    pub fn to_lisp(&self, placeholder: &str) -> String {
        match *self {
            LispExpr::Binary(op, ref lhs, ref rhs) => {
                format!(
                    "({} {} {})",
                    binop_to_string(op),
                    lhs.to_lisp(placeholder),
                    rhs.to_lisp(placeholder)
                )
            }
            LispExpr::Fun(ref name, ref params) => {
                let mut buf = String::new();
                buf.push('(');
                buf.push_str(name);

                for p in params {
                    buf.push(' ');
                    buf.push_str(&p.to_lisp(placeholder));
                }

                buf.push(')');
                buf
            }
            LispExpr::Lit(f) => format!("{}", f),
            LispExpr::Unary(op, ref expr) => {
                format!("({} {})", unop_to_string(op), expr.to_lisp(placeholder))
            }
            LispExpr::Ident(id) => format!("{}{}", placeholder, id),
        }
    }

    pub fn depth(&self) -> u64 {
        match *self {
            LispExpr::Binary(_, ref lhs, ref rhs) => 1 + std::cmp::max(lhs.depth(), rhs.depth()),
            LispExpr::Fun(_, ref params) => 1 + params.iter().map(Self::depth).max().unwrap_or(0),
            LispExpr::Lit(_) => 0,
            LispExpr::Unary(_, ref expr) => expr.depth(),
            LispExpr::Ident(_) => 0,
        }
    }

    pub fn to_rust(&self, cx: &LateContext, bindings: &MatchBindings) -> String {
        fn to_rust_impl(
            expr: &LispExpr,
            cx: &LateContext,
            bindings: &MatchBindings
        ) -> (String, bool) {
            match *expr {
                LispExpr::Binary(op, ref lhs, ref rhs) => {
                    match (to_rust_impl(lhs, cx, bindings), to_rust_impl(rhs, cx, bindings)) {
                        ((lhs, false), (rhs, false)) => {
                            (format!("{} {} {}", lhs, binop_to_string(op), rhs), true)
                        }
                        ((lhs, true), (rhs, false)) => {
                            (format!("({}) {} {}", lhs, binop_to_string(op), rhs), true)
                        }
                        ((lhs, false), (rhs, true)) => {
                            (format!("{} {} ({})", lhs, binop_to_string(op), rhs), true)
                        }
                        ((lhs, true), (rhs, true)) => {
                            (format!("({}) {} ({})", lhs, binop_to_string(op), rhs), true)
                        }
                    }
                }
                LispExpr::Fun(ref name, ref params) => {
                    let mut buf = String::new();
                    match to_rust_impl(&params[0], cx, bindings) {
                        (expr, false) => buf.push_str(&expr),
                        (expr, true) => {
                            buf.push('(');
                            buf.push_str(&expr);
                            buf.push(')');
                        }
                    }
                    buf.push('.');
                    buf.push_str(rust_name(name).unwrap_or("_"));
                    buf.push('(');

                    for (i, p) in params.iter().skip(1).enumerate() {
                        if i != 0 {
                            buf.push_str(", ");
                        }
                        buf.push_str(&to_rust_impl(p, cx, bindings).0);
                    }

                    buf.push(')');
                    (buf, false)
                }
                LispExpr::Lit(f) => (format!("{}", f), false),
                LispExpr::Unary(op, ref expr) => {
                    match to_rust_impl(expr, cx, bindings) {
                        (expr, false) => (format!("{}{}", unop_to_string(op), expr), true),
                        (expr, true) => (format!("{}({})", unop_to_string(op), expr), true),
                    }
                }
                LispExpr::Ident(id) => {
                    match *bindings.bindings.get(&id).expect("Got an unbinded id!") {
                        MatchBinding::Field(_, ref path, ref name) => {
                            (snippet(cx, merge_span(path.span, name.span), "..").into_owned(), false)
                        },
                        MatchBinding::Ident(_, ref path) => {
                            (snippet(cx, path.span, "..").into_owned(), false)
                        }
                        MatchBinding::Lit(_, ref span) => {
                            (snippet(cx, *span, "..").into_owned(), false)
                        }
                        MatchBinding::Other(ref span) => {
                            (snippet(cx, *span, "..").into_owned(), true)
                        }
                        MatchBinding::TupField(_, ref path, ref idx) => {
                            (snippet(cx, merge_span(path.span, idx.span), "..").into_owned(), false)
                        },
                    }
                }
            }
        }

        to_rust_impl(self, cx, bindings).0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParseError {
    Arity,
    Expected(char),
    Ident,
    Float,
    Unexpected(char),
    EOE,
}

#[derive(Debug)]
pub struct Parser {
    ids: Vec<String>,
    stack: Vec<char>,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            ids: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn parse(&mut self, s: &str) -> Result<LispExpr, ParseError> {
        let mut it = s.chars();

        match self.parse_impl(&mut it) {
            Ok(result) => {
                if it.next().is_some() {
                    Err(ParseError::EOE)
                }
                else  {
                    Ok(result)
                }
            }
            err @ Err(..) => err,
        }
    }

    fn parse_impl<It: Iterator<Item = char>>(&mut self, it: &mut It)
    -> Result<LispExpr, ParseError> {
        match self.get_char(it, true) {
            Some('(') => {
                match self.get_char(it, true) {
                    Some('+') => self.parse_op(it, BiAdd),
                    Some('-') => self.parse_op(it, BiSub),
                    Some('*') => self.parse_op(it, BiMul),
                    Some('/') => self.parse_op(it, BiDiv),
                    Some('\u{3bb}') => self.parse_lambda(it),
                    Some(c) => {
                        self.put_back(c);
                        self.parse_fun(it)
                    }
                    None => Err(ParseError::EOE),
                }
            }
            Some(c) if c.is_digit(10) => {
                self.put_back(c);
                self.parse_float(it)
            }
            Some(c) if c.is_alphanumeric() => {
                self.put_back(c);
                self.parse_ident(it)
            }
            Some(c) => {
                self.put_back(c);
                Err(ParseError::Unexpected(c))
            }
            None => Err(ParseError::EOE),
        }
    }

    fn expect<It: Iterator<Item = char>>(
        &mut self,
        it: &mut It,
        c: char,
        skip_whitespace: bool
    ) -> Result<(), ParseError> {
        if self.get_char(it, skip_whitespace) == Some(c) {
            Ok(())
        }
        else {
            Err(ParseError::Expected(c))
        }
    }

    fn parse_float<It: Iterator<Item = char>>(&mut self, it: &mut It)
    -> Result<LispExpr, ParseError> {
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

    fn parse_ident<It: Iterator<Item = char>>(&mut self, it: &mut It)
    -> Result<LispExpr, ParseError> {
        // TODO: Herbie also supports ‘pi’ and ‘e’ as native constants.
        let mut buf = String::new();
        loop {
            let c = self.get_char(it, false);
            if let Some(c) = c {
                if c.is_alphanumeric() {
                    buf.push(c);
                    continue;
                }
                else {
                    self.put_back(c);
                }
            }

            break;
        }

        if let Some(id) = self.ids.iter().position(|e| e == &buf) {
            Ok(LispExpr::Ident(id as u64))
        }
        else {
            self.ids.push(buf);
            Ok(LispExpr::Ident(self.ids.len() as u64 - 1))
        }
    }

    fn parse_lambda<It: Iterator<Item = char>>(&mut self, it: &mut It)
    -> Result<LispExpr, ParseError> {
        loop {
            match it.next() {
                Some(')') | None => break,
                _ => continue,
            }
        }

        let r = self.parse_impl(it);
        try!(self.expect(it, ')', true));
        r
    }

    fn parse_fun<It: Iterator<Item = char>>(&mut self, it: &mut It)
    -> Result<LispExpr, ParseError> {
        let mut buf = String::new();
        loop {
            let c = self.get_char(it, false);
            if let Some(c) = c {
                if c.is_alphanumeric() {
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
            if let Ok(idx) = KNOWN_FUNS.binary_search_by(|p| p.0.cmp(&buf)) {
                return if KNOWN_FUNS[idx].2 == params.len() {
                    Ok(LispExpr::Fun(buf, params))
                }
                else {
                    Err(ParseError::Arity)
                };
            }
            else if buf == "sqr" && params.len() == 1 {
                return Ok(LispExpr::Binary(BiMul, box params[0].clone(), box params.remove(0)));
            }
        }

        Err(ParseError::Ident)
    }

    fn parse_op<It: Iterator<Item = char>>(&mut self, it: &mut It, op: BinOp_)
    -> Result<LispExpr, ParseError> {
        // TODO: Herbie seems to also support the following for the repip of a float: (/ 42) and
        // rust has a function recip for that
        let lhs = try!(self.parse_impl(it));
        let r = if let Ok(rhs) = self.parse_impl(it) {
            Ok(LispExpr::Binary(op, box lhs, box rhs))
        }
        else if op == BiSub {
            Ok(LispExpr::Unary(UnNeg, box lhs))
        }
        else {
            return Err(ParseError::Arity);
        };
        try!(self.expect(it, ')', true));
        r
    }

    fn get_char<It: Iterator<Item = char>>(
        &mut self,
        it: &mut It,
        skip_whitespace: bool
    ) -> Option<char> {
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
