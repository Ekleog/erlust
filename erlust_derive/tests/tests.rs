#![feature(proc_macro_non_items)]

extern crate erlust_derive;

use erlust_derive::receive;

fn foo(a: usize) -> usize {
    a + 1
}

#[test]
fn non_stupid() {
    assert_eq!(
        42,
        receive!(
        (usize, String): (1, s) => foo(s),
        usize: x if bar(x) => { baz(x) }
    )
    );
}
