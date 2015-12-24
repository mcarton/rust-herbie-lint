#![feature(plugin)]
#![plugin(herbie_lint)]

#[deny(herbie)]

fn main() {
    (0/1 + 2) * 1; //~ERROR
}
