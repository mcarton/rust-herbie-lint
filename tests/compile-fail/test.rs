#![feature(plugin)]
#![plugin(herbie_lint)]

#![deny(herbie)]

fn foo() -> f64 {
    4.2
}

fn main() {
    let (a, b, c) = (0., 0., 0.);
    (a/a + a) * a; //~ERROR
    (a/b + a) * b; //~ERROR
    (a/b + c) * b; //~ERROR

    (a/b + c) * a;

    (0./1. + 2.) * 1.; //~ERROR
    (1./1. + 2.) * 1.; //~ERROR
    (1./1. + 1.) * 1.; //~ERROR

    (0./1. + a) * 1.; //~ERROR
    (0./a + 2.) * a; //~ERROR

    (a/b + foo()) * b; //~ERROR
    (a/b + (42 as f64)) * b; //~ERROR
    (a/b + { 42. }) * b; //~ERROR

    (a/foo() + c) * foo();
    (a/{ 42. } + c) * { 42. };
}
