#![feature(plugin)] //~ERROR: Could not initialize Herbie-Lint
#![plugin(herbie_lint)]

#![allow(unused_variables)]
#![deny(herbie)]

fn main() {
    let a = 42.;
}
