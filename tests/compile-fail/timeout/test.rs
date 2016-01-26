#![feature(plugin)]
#![plugin(herbie_lint)]

#![deny(herbie, unused_variables)]

fn main() {
    let a = 0.;
    let b = 0.;

    b * ((a - 1.)/a);
    //~^ NOTE Calling Herbie on the following expression, it might take a while
    //~| NOTE timed out

    // Just so there actually is an error in that file for compiletest_rs
    let c = 1; //~ERROR unused variable
}
