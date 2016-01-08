#![feature(box_syntax)]
#![feature(plugin_registrar)]
#![feature(rustc_private)]

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
)]

extern crate itertools;
extern crate rusqlite;
#[macro_use]
extern crate rustc;
#[macro_use]
extern crate rustc_plugin;
#[macro_use]
extern crate rustc_front;
extern crate rustc_serialize;
extern crate syntax;
extern crate toml;

use rustc_plugin::Registry;

mod conf;
pub mod lint;
pub mod lisp;
mod utils;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_late_lint_pass(box lint::Herbie::new());
}
