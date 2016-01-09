extern crate compiletest_rs as compiletest;

use std::env::{current_dir, set_current_dir, set_var, var};
use std::fs::{copy, read_dir, remove_file};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

    let db_orig = Path::new("Herbie.orig.db");
    let has_db_orig = db_orig.exists();

    if has_db_orig {
        copy(db_orig, "Herbie.db").is_ok();
    }

    set_var("PATH", format!("{}:.", var("PATH").unwrap()));

    compiletest::run_tests(&config);

    if has_db_orig {
        let Output { status, stdout, stderr } =
            Command::new("sqldiff")
            .arg("Herbie.dest.db").arg("Herbie.db")
            .output()
            .unwrap()
        ;

        assert!(status.success());
        assert!(stdout.is_empty(), "DB have diff:\n>>>\n{}<<<", String::from_utf8_lossy(&stdout));
        assert!(stderr.is_empty(), "{}", String::from_utf8_lossy(&stderr));

        remove_file("Herbie.db").unwrap();
    }
}

#[test]
fn compile_test() {
    let cwd = current_dir().unwrap();
    let target_dir = cwd.join("target/debug/");

    for dir in read_dir("tests/compile-fail").unwrap() {
        let dir = cwd.join(dir.unwrap().path());
        run_mode("compile-fail", dir, &target_dir);
    }
}
