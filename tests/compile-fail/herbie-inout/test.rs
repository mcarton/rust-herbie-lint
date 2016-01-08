#![feature(plugin)]
#![plugin(herbie_lint)]

#![allow(unused_variables)]
#![deny(herbie)]

fn main() {
    let a = 0.;
    let b = 0.;

    b * ((a - 1.)/a);
    //~^ NOTE Calling Herbie on the following expression, it might take a while
    //~| ERROR
    //~| HELP Try this
    //~| SUGGESTION b - (b / a);
}
