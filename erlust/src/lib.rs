#[macro_use]
extern crate futures;
extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

use futures::{future, Future};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    sync::atomic::{AtomicUsize, Ordering},
};

static CUR_ACTOR_ID: AtomicUsize = AtomicUsize::new(0);

task_local! {
    static ACTOR_ID: usize = CUR_ACTOR_ID.fetch_add(1, Ordering::SeqCst)
}

pub struct Pid {
    // TODO: (A) Cross-process / over-the-network messages
    actor_id: usize,
}

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] is a bare minimum, and it's recommended
// to include a struct name and maybe even version field in the serialized
// data.
pub trait Message: 'static + Serialize + for<'a> Deserialize<'a> {}

pub enum Void {}

pub fn send<M: Message>(_to: Pid, _msg: M) {
    unimplemented!()
}

pub fn receive(_wanted: &Fn(&Any) -> bool) -> impl Future<Item = Box<Any + 'static>, Error = Void> {
    unimplemented!();
    future::ok(Box::new(()) as Box<Any>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestMsg {
        foo: String,
    }
    impl Message for TestMsg {}
}
