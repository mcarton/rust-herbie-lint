extern crate compiletest_rs as compiletest;

use std::env::{current_dir, set_current_dir, var};
use std::fs::read_dir;
use std::path::{Path, PathBuf};

fn run_mode(mode: &'static str, dir: PathBuf, target_dir: &Path) {
    let mut config = compiletest::default_config();

    config.target_rustcflags = Some(format!("-L {}", target_dir.to_str().unwrap()));

    if let Ok(name) = var::<&str>("TESTNAME") {
        config.filter = Some(name.to_owned())
    }

    let cfg_mode = mode.parse().ok().expect("Invalid mode");
    config.mode = cfg_mode;

    set_current_dir(&dir).unwrap();
    config.src_base = current_dir().unwrap();

    compiletest::run_tests(&config);
}

#[test]
fn compile_test() {
    let cwd = current_dir().unwrap();
    let target_dir = cwd.join("target/debug/");

    for dir in read_dir("tests/compile-fail").unwrap() {
        let dir = cwd.join(dir.unwrap().path());
        println!(">>{:?}<<", dir);
        run_mode("compile-fail", dir, &target_dir);
    }
}
