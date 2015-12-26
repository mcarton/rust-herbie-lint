use rustc::lint::LintContext;
use std::borrow::Cow;
use syntax::codemap::{Span, mk_sp};

/// Convert a span to a code snippet if available, otherwise use default, e.g.
/// `snippet(cx, expr.span, "..")`.
/// From clippy.
pub fn snippet<'a, T: LintContext>(cx: &T, span: Span, default: &'a str) -> Cow<'a, str> {
    cx.sess().codemap().span_to_snippet(span).map(From::from).unwrap_or(Cow::Borrowed(default))
}

/// Merge tow spans.
pub fn merge_span(begin: Span, end: Span) -> Span {
    mk_sp(begin.lo, end.hi)
}
