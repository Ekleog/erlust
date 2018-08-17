#![feature(
    arbitrary_self_types,
    async_await,
    await_macro,
    futures_api,
    pin
)]

extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

mod local_channel;
mod local_channel_updater;
mod local_senders;
mod types;

use futures::prelude::*;
use serde::{Deserialize, Serialize};
use std::mem;

use self::{
    local_channel::{LocalChannel, MY_CHANNEL},
    local_channel_updater::LocalChannelUpdater,
    local_senders::LOCAL_SENDERS,
    types::{ActorId, LocalMessage, LocalReceiver, LocalSender},
};

pub use futures::{channel::mpsc::SendError, task::SpawnError};

pub fn spawn<Fut>(fut: Fut) -> impl Future<Output = Result<(), SpawnError>>
where
    Fut: Future<Output = ()> + Send + 'static,
{
    let task = LocalChannelUpdater::new(fut);
    future::lazy(move |cx| cx.spawner().spawn(task))
}

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] is a bare minimum, and it's recommended
// to include a struct name and maybe even version field in the serialized
// data.
pub trait Message: 'static + Send + Serialize + for<'de> Deserialize<'de> {
    fn tag() -> &'static str;
}

pub struct Pid {
    // TODO: (A) Cross-process / over-the-network messages
    // TODO: (A) Add sender of the message to each received message
    actor_id: ActorId,
    sender:   Option<LocalSender>,
}

impl Pid {
    pub fn me() -> Pid {
        let (actor_id, sender) = MY_CHANNEL.with(|c| {
            let cell = c.borrow();
            let chan = cell.as_ref().unwrap();
            (chan.actor_id, Some(chan.sender.clone()))
        });
        Pid { actor_id, sender }
    }

    pub async fn send<M: Message>(&mut self, msg: Box<M>) -> Result<(), SendError> {
        if let Some(ref mut sender) = self.sender {
            await!(sender.send(msg as LocalMessage))
        } else {
            // TODO: (C) Check these `.unwrap()` are actually sane
            let mut sender = LOCAL_SENDERS.read().unwrap().get(self.actor_id).unwrap();
            await!(sender.send(msg as LocalMessage))
        }
    }
}

pub enum ReceiveResult<Ret> {
    Use(Ret),
    Skip(LocalMessage),
}

pub async fn receive<HandleFn, Fut, Ret>(handle: HandleFn) -> Ret
where
    Fut: Future<Output = ReceiveResult<Ret>>,
    HandleFn: Fn(LocalMessage) -> Fut,
{
    use self::ReceiveResult::*;

    // This `expect` shouldn't trigger, because `LocalChannelUpdater` should always
    // keep `MY_CHANNEL` task-local. As such, the only moment where it should be
    // set to `None` is here, and it is restored before the end of this function,
    // and `__receive` cannot be called inside `__receive`.
    let mut chan = MY_CHANNEL
        .with(|c| c.borrow_mut().take())
        .expect("Called receive inside receive");;

    // First, attempt to find a message in waiting list
    for i in 0..chan.waiting.len() {
        // TODO: (C) consider unsafe here to remove the allocation, dep. on benchmarks
        let mut msg = Box::new(()) as LocalMessage;
        mem::swap(&mut msg, &mut chan.waiting[i]);
        match await!(handle(msg)) {
            Use(ret) => {
                chan.waiting.remove(i);
                MY_CHANNEL.with(|c| *c.borrow_mut() = Some(chan));
                return ret;
            }
            Skip(msg) => {
                chan.waiting[i] = msg;
            }
        }
    }

    // Push all irrelevant messages to the waiting list, then return relevant one
    loop {
        // This `expect` shouldn't trigger, because `chan.receiver.next()` is
        // supposed to answer `None` iff all `Sender`s associated to the channel
        // have been dropped. Except we always keep a `Sender` alive in the
        // `LOCAL_SENDERS` map, and `__receive` should not be able to be called
        // once the actor has been dropped, so this should be safe.
        let msg = await!(chan.receiver.next()).expect("Called receive after the actor was dropped");
        match await!(handle(msg)) {
            Use(ret) => {
                MY_CHANNEL.with(|c| *c.borrow_mut() = Some(chan));
                return ret;
            }
            Skip(msg) => {
                chan.waiting.push_back(msg);
            }
        }
    }
}

// TODO: (A) add a registry to record name<->Pid associations?

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestMsg {
        foo: String,
    }
    impl Message for TestMsg {
        fn tag() -> &'static str {
            "test"
        }
    }

    // TODO: (A) Add receive! / receive_box! tests
}
