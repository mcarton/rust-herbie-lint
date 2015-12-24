use rustc::lint::{LateContext, LintArray, LintPass, LateLintPass};
use rustc_front::hir::*;

pub struct Herbie;

declare_lint!(pub HERBIE, Warn,
              "checks for numerical instability");

impl LintPass for Herbie {
    fn get_lints(&self) -> LintArray {
        lint_array!(HERBIE)
    }
}

impl LateLintPass for Herbie {
}
