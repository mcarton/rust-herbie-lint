#![feature(plugin)]
#![plugin(herbie_lint)]

#![deny(herbie)]

struct Foo { a: f64, b: f64 }

fn get_f64() -> f64 {
    4.2
}

fn get_tup() -> (f64, f64) {
    (0., 0.)
}

fn get_struct() -> Foo {
    Foo { a: 0., b: 0. }
}

fn integers() {
    1;
    (1/2 + 3) * 2;
}

fn main() {
    integers();

    let (a, b, c) = (0., 0., 0.);

    a*a + b*b;

    (a/a + a) * a;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (a * a) + a
    (a/b + a) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (a * b) + a
    (a/b + c) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (c * b) + a

    (a/b + c) * a;

    (0./1. + 2.) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (2. * 1.) + 0.
    (1./1. + 2.) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (2. * 1.) + 1.
    (1./1. + 1.) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (1. * 1.) + 1.

    (0./1. + a) * 1.;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (a * 1.) + 0.
    (0./a + 2.) * a;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (2. * a) + 0.

    (a/b + get_f64()) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION ((get_f64()) * b) + a
    (a/b + (4.5f64).sqrt()) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (((4.5f64).sqrt()) * b) + a
    (a/b + (42 as f64)) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION ((42 as f64) * b) + a
    (a/b + { 42. }) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (({ 42. }) * b) + a

    (a/get_f64() + c) * get_f64();
    (a/{ 42. } + c) * { 42. };

    (4.5f64).abs();

    //(a*a + b*b).sqrt();
    //**^ ERROR
    //**| HELP Try this
    //**| SUGGESTION a.hypot(b)

    //(a*a + (-4. * -4.)).sqrt();
    //**^ ERROR
    //**| HELP Try this
    //**| SUGGESTION a.hypot(-4.))

    a.floor();

    (a/b + c.floor()) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION ((c.floor()) * b) + a

    let d = (0., 0.);

    (a/b + d.0) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (d.0 * b) + a

    (a/b + get_tup().0) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION ((get_tup().0) * b) + a

    (a/d.0 + c) * d.0;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (c * d.0) + a

    (a/d.0 + c) * d.1;
    (a/get_tup().0 + c) * get_tup().0;

    let e = get_struct();

    (a/b + e.a) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (e.a * b) + a

    (a/b + get_struct().a) * b;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION ((get_struct().a) * b) + a

    (a/e.a + c) * e.a;
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (c * e.a) + a

    (a/get_struct().a + c) * get_struct().a;
    (a/e.a + c) * e.b;

    1. - a.cos();
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (a.sin() * a.sin()) / a.cos().log1p().exp()

    (a+b).sin() - (a+b).cos();
    //~^ ERROR
    //~| HELP Try this
    //~| SUGGESTION (b.sin() * (a.sin() + a.cos())) - ((a.cos() - a.sin()) * b.cos())
}
