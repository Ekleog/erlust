#![feature(async_await, await_macro, proc_macro_non_items)]

#[macro_use]
extern crate erlust_derive;
#[macro_use]
extern crate serde_derive;

use erlust_derive::receive;

fn foo(_: &String) -> bool {
    false
}

fn bar(x: &String) -> String {
    x.clone()
}

fn foobar(x: String) -> String {
    x
}

fn baz(_: usize) -> bool {
    true
}

fn quux(x: usize) -> String {
    format!("{}", x)
}

// TODO: (B) this example is actually stupid, its name refers to the macro
#[test]
fn non_stupid() {
    async {
        assert_eq!(
            "test",
            receive!(
                (usize, String): (1, ref x) if foo(x) => bar(x),
                (usize, String): (2, x) => foobar(x),
                usize: x if baz(x) => quux(x),
            )
        );
    };
}

#[derive(Deserialize, Message, Serialize)]
#[erlust_tag = "hello"]
struct FooBar {
    hello: usize,
}
