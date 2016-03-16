#![feature(plugin)]
//~^ERROR: Could not initialize Herbie-Lint
//~^^NOTE: Configuration error: Syntax error in Herbie.toml
#![plugin(herbie_lint)]

#![allow(unused_variables)]
#![deny(herbie)]
//~^NOTE: lint level defined here

fn main() {
    let a = 42.;
}
