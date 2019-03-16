#![feature(async_await, await_macro, futures_api, proc_macro_hygiene, stmt_expr_attributes)]

#[macro_use]
extern crate erlust_derive;
#[macro_use]
extern crate serde_derive;

use erlust_derive::receive;

#[derive(Deserialize, Message, Serialize)]
#[erlust_tag = "foo"]
struct Foo(usize, String);

#[derive(Deserialize, Message, Serialize)]
#[erlust_tag = "bar"]
struct Bar(usize);

fn foo(_: &String) -> bool {
    false
}

fn bar(x: String) -> String {
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
                Foo: (_pid, Foo(1, x)) if foo(x) => bar(x),
                Foo: (_pid, Foo(2, x)) => foobar(x),
                Bar: (_pid, Bar(x)) if baz(*x) => quux(x),
            )
        );
    };
}

#[derive(Deserialize, Message, Serialize)]
#[erlust_tag = "hello"]
struct FooBar {
    hello: usize,
}
