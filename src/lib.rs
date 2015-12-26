#![feature(box_syntax)]
#![feature(plugin_registrar)]
#![feature(rustc_private)]

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate rusqlite;
#[macro_use]
extern crate rustc;
#[macro_use]
extern crate rustc_plugin;
#[macro_use]
extern crate rustc_front;
extern crate syntax;

use rustc_plugin::Registry;

pub mod lint;
mod lisp;
mod utils;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    if let Ok(herbie) = lint::Herbie::new() {
        reg.register_late_lint_pass(box herbie);
    }
}
