#![feature(box_syntax, rustc_private)]

#[allow(plugin_as_library)]
extern crate herbie_lint;
extern crate rustc_front;

use herbie_lint::lisp::{LispExpr, Parser, ParseError};
use herbie_lint::lisp::LispExpr::*;
use rustc_front::hir::BinOp_::*;
use rustc_front::hir::UnOp::*;

pub fn parse(s: &str) -> Result<LispExpr, ParseError> {
    let mut parser = Parser::new();
    parser.parse(s)
}

#[test]
fn test_parser() {
    assert_eq!(parse(""), Err(ParseError::EOE));

    assert_eq!(parse("0."), Ok(Lit(0.)));
    assert_eq!(parse("herbie"), Ok(Ident(0)));
    assert_eq!(parse("(+ 0. herbie1)"), Ok(Binary(BiAdd, box Lit(0.), box Ident(0))));

    let valid = &[
        ("0", Lit(0.)),
        ("herbie0", Ident(0)),
        ("(+ herbie0 herbie1)", Binary(BiAdd, box Ident(0), box Ident(1))),
        ("(- herbie0)", Unary(UnNeg, box Ident(0))),
        ("(- herbie0 herbie1)", Binary(BiSub, box Ident(0), box Ident(1))),
        ("(cos 1)", Fun("cos".into(), vec![Lit(1.)])),
        ("(cos herbie0)", Fun("cos".into(), vec![Ident(0)])),
        ("(log1p (cos herbie0))", Fun("log1p".into(), vec![Fun("cos".into(), vec![Ident(0)])])),
    ];

    for &(s, ref e) in valid {
        assert_eq!(parse(s).as_ref(), Ok(e));
        assert_eq!(s, e.to_lisp("herbie"));
    }


    assert_eq!(parse("(+ 0. 0.) foobar"), Err(ParseError::EOE));
    assert_eq!(parse("("), Err(ParseError::EOE));
    assert_eq!(parse("0.eee"), Err(ParseError::Float));


    assert_eq!(parse("+"), Err(ParseError::Unexpected('+')));
    assert_eq!(parse("(+)"), Err(ParseError::Unexpected(')')));
    assert_eq!(parse("(+ 0.)"), Err(ParseError::Arity));
    assert_eq!(parse("(+ 0. 0."), Err(ParseError::Expected(')')));
    assert_eq!(parse("(+ 0. 0. 0.)"), Err(ParseError::Expected(')')));
    assert_eq!(parse("(+ 0. 0. 0."), Err(ParseError::Expected(')')));

    assert_eq!(parse("-"), Err(ParseError::Unexpected('-')));
    assert_eq!(parse("(-)"), Err(ParseError::Unexpected(')')));
    assert_eq!(parse("(- 0. 0. 0.)"), Err(ParseError::Expected(')')));


    assert_eq!(parse("(cos)"), Err(ParseError::Arity));
    assert_eq!(parse("(cos"), Err(ParseError::Expected(')')));
    assert_eq!(parse("(cos 1. 2.)"), Err(ParseError::Arity));
    assert_eq!(parse("(cos 1. 2. 3."), Err(ParseError::Expected(')')));

    assert_eq!(parse("(foo 1.)"), Err(ParseError::Ident));
    assert_eq!(parse("(foocos 1.)"), Err(ParseError::Ident));
    assert_eq!(parse("(cosfoocos 1.)"), Err(ParseError::Ident));

    let mut parser = Parser::new();
    assert_eq!(parser.parse("(* (+ (/ herbie0 herbie1) herbie2) herbie1)"), Ok(
        Binary(BiMul,
            box Binary(BiAdd,
                box Binary(BiDiv, box Ident(0), box Ident(1)),
                box Ident(2)
            ),
            box Ident(1)
        )
    ));

    assert_eq!(parser.parse("(+ (* herbie2 herbie1) herbie0)"), Ok(
        Binary(BiAdd,
            box Binary(BiMul, box Ident(2), box Ident(1)),
            box Ident(0)
        )
    ));

    assert_eq!(parse("(+ (* herbie2 herbie1) herbie0)"), Ok(
        Binary(BiAdd,
            box Binary(BiMul, box Ident(0), box Ident(1)),
            box Ident(2)
        )
    ));
}
