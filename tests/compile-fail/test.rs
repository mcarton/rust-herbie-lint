#![feature(plugin)]
#![plugin(herbie_lint)]

#![deny(herbie)]

fn foo() -> f64 {
    4.2
}

fn main() {
    let (a, b, c) = (0., 0., 0.);
    (a/a + a) * a;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
    (a/b + a) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
    (a/b + c) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)

    (a/b + c) * a;

    (0./1. + 2.) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
    (1./1. + 2.) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
    (1./1. + 1.) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)

    (0./1. + a) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
    (0./a + 2.) * a;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)

    (a/b + foo()) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
    (a/b + (42 as f64)) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
    (a/b + { 42. }) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)

    (a/foo() + c) * foo();
    (a/{ 42. } + c) * { 42. };

    (4.5f64).abs();

    (a*a + b*b).sqrt();
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (hypot $0 $1)

    ((a-b) * (a-b)).sqrt();
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (- $0 $1)

    a.floor();

    (a/b + c.floor()) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (+ (* $2 $1) $0)
}
