# *Herbie* lint for Rust [![Build Status][travis-svg]][travis] [![Crates.io][crate-svg]][crate] [![License][license-svg]][license]

## What

This plugin can add warnings or errors to your crate when using a numerically
unstable floating point expression.

Quick example of what you can get when compiling
[`tests/compile-fail/general/test.rs`][example]:
```rust
test.rs:40:5: 40:18 warning: Numerically unstable expression, #[warn(herbie)] on by default
test.rs:40     (a/b + c) * b;
               ^~~~~~~~~~~~~
test.rs:40:5: 40:18 help: Try this
test.rs:       (c * b) + a;
test.rs:67:5: 67:23 warning: Numerically unstable expression, #[warn(herbie)] on by default
test.rs:67     (a*a + b*b).sqrt();
               ^~~~~~~~~~~~~~~~~~
test.rs:67:5: 67:23 help: Try this
test.rs:       a.hypot(b);
test.rs:155:5: 155:30 warning: Numerically unstable expression, #[warn(herbie)] on by default
test.rs:155     (a+b).sin() - (a+b).cos();
                ^~~~~~~~~~~~~~~~~~~~~~~~~
test.rs:155:5: 155:30 help: Try this
test.rs:        (b.sin() * (a.sin() + a.cos())) - ((a.cos() - a.sin()) * b.cos());
```

As you can see, it will report numerically unstable expressions, and suggest a
(sometimes over-parenthesized) more stable correction.

## Usage
### Plugin
This is a `rustc` plugin, to use it, you need a *nightly* Rust.

You need a database of possible corrections for this plugin to work. The
database format is the same as [Herbie GHC Plugin (for Haskell)][ghc-herbie]
from which this plugin is inspired so [this file][ghc-herbie-db] should work.
Just put it in the same directory you call `cargo` or `rustc` from.

Add in your *Cargo.toml*:

```toml
[dependencies]
herbie-lint = "{{VERSION}}"
```

and in your crate:

```rust
#![feature(plugin)]
#![plugin(herbie_lint)]
```

See [*clippy*][clippy]'s [*Usage* section][clippy-usage] if you want to know
more and if you want more Rust lints.

### Configuration
If you don't want the plugin to lint a particular function or method, you can
mark it with the `#[herbie_ignore]` attribute:

```rust
fn foo(a: f64, b: f64, c: f64) -> f64 {
    (a/b + c) * b
    // This will suggest to use “(c * b) + a” instead.
}

#[herbie_ignore]
fn bar(a: f64, b: f64, c: f64) -> f64 {
    (a/b + c) * b
    // This won't.
}
```

You can also put a `Herbie.toml` file next to your `Cargo.toml` with the
following fields:
```toml
# Path to the database.
db_path = "Herbie.db"

# The seed use by Herbie. If not provided, a fixed seed will be used. Fixing
# the seed ensures deterministic builds.
herbie_seed = "#(1461197085 2376054483 1553562171 1611329376 2497620867 2308122621)"

# Allow the plugin to call Herbie on unknown expressions. Positive results from
# Herbie will be cached in the database. WARNING: Herbie is slow.
# If ‘true’, the plugin will fail if it cannot find the executable.
# If ‘false’, the plugin will not try to run Herbie.
# By default, the plugin will call the executable only if it's found, but won't
# complain otherwise.
use_herbie = false

# Maximum time in seconds that Herbie is allowed to play with an expression. If
# null, allow Herbie to run indefinitely. Default is two minutes.
timeout = 120
```

More information about calling Herbie can be found in the
[wiki][wiki-herbie-inout].

## Acknowledgment
Thanks to @llogiq for [the idea][idea].

[clippy-usage]: https://github.com/Manishearth/rust-clippy#usage
[clippy]: https://github.com/Manishearth/rust-clippy
[crate-svg]: https://img.shields.io/crates/v/herbie-lint.svg
[crate]: https://crates.io/crates/herbie-lint/
[example]: https://github.com/mcarton/rust-herbie-lint/blob/master/tests/compile-fail/general/test.rs
[ghc-herbie-db]: https://github.com/mikeizbicki/HerbiePlugin/blob/master/data/Herbie.db?raw=true
[ghc-herbie]: https://github.com/mikeizbicki/HerbiePlugin
[idea]: https://github.com/Manishearth/rust-clippy/issues/346
[license-svg]: https://img.shields.io/crates/l/herbie-lint.svg
[license]: https://github.com/mcarton/rust-herbie-lint/blob/master/LICENSE
[travis-svg]: https://travis-ci.org/mcarton/rust-herbie-lint.svg
[travis]: https://travis-ci.org/mcarton/rust-herbie-lint/
[wiki-herbie-inout]: https://github.com/mcarton/rust-herbie-lint/wiki#how-to-use-herbie-optional
