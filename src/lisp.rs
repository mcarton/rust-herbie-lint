#![allow(float_cmp)]

use std;
use rustc_front::hir::*;
use syntax::ast::Lit_::*;
use syntax::ast::FloatTy;
use rustc_front::util::{binop_to_string, unop_to_string};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

pub enum LispExpr {
    Binary(BinOp_, Box<LispExpr>, Box<LispExpr>),
    Fun(String, Vec<LispExpr>),
    Ident(u64),
    Lit(f64),
    Unary(UnOp, Box<LispExpr>),
}

#[derive(Debug)]
pub enum LispExprError {
    UnknownType,
    UnknownKind,
    WrongFloat,
}

impl std::fmt::Debug for LispExpr {

    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.pad(&self.to_lisp())
    }

}

impl LispExpr {

    pub fn form_of(&self, other: &LispExpr) -> bool {
        #[derive(PartialEq)]
        enum Binded {
            Ident(u64),
            Lit(f64),
        }

        fn form_of_impl(lhs: &LispExpr, rhs: &LispExpr, ids: &mut HashMap<u64, Binded>) -> bool {
            match (lhs, rhs) {
                (&LispExpr::Binary(lop, ref lp1, ref lp2), &LispExpr::Binary(rop, ref rp1, ref rp2)) => {
                    lop == rop && form_of_impl(lp1, rp1, ids) && form_of_impl(lp2, rp2, ids)
                },
                (&LispExpr::Fun(ref lfun, ref lp), &LispExpr::Fun(ref rfun, ref rp)) => {
                    lfun == rfun && lp.len() == rp.len() && lp.iter().zip(rp).all(|(lp, rp)| form_of_impl(lp, rp, ids))
                },
                (&LispExpr::Ident(lid), &LispExpr::Ident(rid)) => {
                    match ids.entry(rid) {
                        Entry::Occupied(entry) => {
                            if let Binded::Ident(binded) = *entry.get() {
                                binded == lid
                            }
                            else {
                                false
                            }
                        },
                        Entry::Vacant(vacant) => {
                            vacant.insert(Binded::Ident(lid));
                            true
                        }
                    }
                },
                (&LispExpr::Lit(l), &LispExpr::Lit(r)) => {
                    l == r
                },
                (&LispExpr::Lit(l), &LispExpr::Ident(rid)) => {
                    match ids.entry(rid) {
                        Entry::Occupied(entry) => {
                            if let Binded::Lit(binded) = *entry.get() {
                                binded == l
                            }
                            else {
                                false
                            }
                        },
                        Entry::Vacant(vacant) => {
                            vacant.insert(Binded::Lit(l));
                            true
                        }
                    }
                },
                (&LispExpr::Unary(lop, ref lp), &LispExpr::Unary(rop, ref rp)) => {
                    lop == rop && form_of_impl(lp, rp, ids)
                },
                _ => false,
            }
        }

        let mut ids = HashMap::new();
        form_of_impl(self, other, &mut ids)
    }

    pub fn from_expr(expr: &Expr) -> Result<LispExpr, LispExprError> {
        match expr.node {
            ExprBinary(op, ref lhs, ref rhs) => {
                Ok(LispExpr::Binary(op.node, box try!(Self::from_expr(lhs)), box try!(Self::from_expr(rhs))))
            },
            ExprLit(ref lit) => {
                match lit.node {
                    LitFloat(ref f, FloatTy::TyF64) => Self::from_lit_float(&f),
                    LitFloatUnsuffixed(ref f) => Self::from_lit_float(&f),
                    _ => Err(LispExprError::UnknownType)
                }
            },
            ExprUnary(op, ref expr) => {
                Ok(LispExpr::Unary(op, box try!(Self::from_expr(&expr))))
            },
            // TODO:
            // ExprCall,
            // ExprMethodCall,
            // ExprCast,
            // ExprBlock,
            // ExprAssignOp,
            // ExprField,
            // ExprTupField,
            // ExprIndex,
            // ExprPath,
            _ => Err(LispExprError::UnknownKind)
        }
    }

    fn from_lit_float(f: &str) -> Result<LispExpr, LispExprError> {
        if let Ok(f) = f.parse() {
            Ok(LispExpr::Lit(f))
        }
        else {
            Err(LispExprError::WrongFloat)
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
            }
            LispExpr::Lit(f) => {
                format!("{}", f)
            },
            LispExpr::Unary(op, ref expr) => {
                format!("({} {})", unop_to_string(op), expr.to_lisp())
            },
            LispExpr::Ident(id) => {
                format!("${}", id)
            }
        }
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
                    Some(c) if c.is_digit(10) => {
                        self.put_back(c);
                        let r = self.parse_float(it);
                        try!(self.expect(it, ')', true));
                        r
                    },
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
