# *Herbie* lint for Rust [![Build Status][travis-svg]][travis] [![Crates.io][crate-svg]][crate] [![License][license-svg]][license] [![Coverage][coverage-svg]][coverage]

## What

This plugin can add warnings or errors to your crate when using a numerically
instable floating point expression.

Quick example of what you can get when compiling `test/compile-fail/test.rs`:
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

As you can see, it will report numerically instable expressions, and suggest a
(sometimes over-parenthesized) more stable correction.

## Usage
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
#![plugin(clippy)]
```

See [*clippy*][clippy]'s [*Usage* section][clippy-usage] if you want to know
more and if you want more Rust lints.

[clippy-usage]: https://github.com/Manishearth/rust-clippy#usage
[clippy]: https://github.com/Manishearth/rust-clippy
[coverage-svg]: https://coveralls.io/repos/mcarton/rust-herbie-lint/badge.svg?branch=master&service=github
[coverage]: https://coveralls.io/github/mcarton/rust-herbie-lint/
[crate-svg]: https://img.shields.io/crates/v/herbie-lint.svg
[crate]: https://crates.io/crates/herbie-lint/
[ghc-herbie-db]: https://github.com/mikeizbicki/HerbiePlugin/blob/master/data/Herbie.db?raw=true
[ghc-herbie]: https://github.com/mikeizbicki/HerbiePlugin
[license-svg]: https://img.shields.io/crates/l/herbie-lint.svg
[license]: https://github.com/mcarton/rust-herbie-lint/blob/master/LICENSE
[travis-svg]: https://travis-ci.org/mcarton/rust-herbie-lint.svg
[travis]: https://travis-ci.org/mcarton/rust-herbie-lint/
