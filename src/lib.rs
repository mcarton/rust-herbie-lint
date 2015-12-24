#![feature(plugin_registrar)]
#![feature(rustc_private)]

#[macro_use]
extern crate rustc;
#[macro_use]
extern crate rustc_plugin;
#[macro_use]
extern crate rustc_front;

use rustc_plugin::Registry;

pub mod lint;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_late_lint_pass(Box::new(lint::Herbie));
}
