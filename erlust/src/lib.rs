#![feature(arbitrary_self_types, async_await, await_macro, futures_api, pin)]

extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

use futures::prelude::*;
use futures::channel::mpsc;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, LinkedList},
    mem::PinMut,
    sync::RwLock,
};

pub use futures::{channel::mpsc::SendError, task::SpawnError};

const QUEUE_BUFFER: usize = 64; // TODO: (C) fiddle

type ActorId = usize;
type LocalMessage = Box<Send + 'static>;
type LocalSender = mpsc::Sender<LocalMessage>;
type LocalReceiver = mpsc::Receiver<LocalMessage>;

struct LocalSenders {
    next_actor_id: ActorId,
    map: HashMap<ActorId, LocalSender>,
}

impl LocalSenders {
    fn new() -> LocalSenders {
        LocalSenders {
            next_actor_id: 0,
            map: HashMap::new(),
        }
    }

    fn allocate(&mut self, sender: LocalSender) -> ActorId {
        let actor_id = self.next_actor_id;
        // TODO: (C) try to handle gracefully the overflow case
        self.next_actor_id = self.next_actor_id.checked_add(1).unwrap();
        self.map.insert(actor_id, sender);
        actor_id
    }

    fn get(&self, actor_id: ActorId) -> Option<LocalSender> {
        self.map.get(&actor_id).map(|s| s.clone())
    }
}

lazy_static! {
    static ref LOCAL_SENDERS: RwLock<LocalSenders> = RwLock::new(LocalSenders::new());
}

struct LocalChannel {
    actor_id: ActorId,
    receiver: LocalReceiver,
    waiting:  LinkedList<LocalMessage>, // TODO: (C) Evaluate whether Vec wouldn't be better
}

impl LocalChannel {
    fn new() -> LocalChannel {
        let (sender, receiver) = mpsc::channel(QUEUE_BUFFER);
        // TODO: (A) make async (qutex + https://github.com/rust-lang-nursery/futures-rs/issues/1187 ? or ragequit and use parking_lot?)
        let actor_id = LOCAL_SENDERS.write().unwrap().allocate(sender);
        LocalChannel {
            actor_id,
            receiver,
            waiting: LinkedList::new(),
        }
    }
}

thread_local! {
    static MY_CHANNEL: RefCell<Option<LocalChannel>> = RefCell::new(None);
}

struct LocalChannelUpdater<Fut: Future<Output = ()>> {
    channel: Option<LocalChannel>,
    fut: Fut,
}

impl<Fut: Future<Output = ()>> LocalChannelUpdater<Fut> {
    fn new(fut: Fut) -> LocalChannelUpdater<Fut> {
        LocalChannelUpdater {
            channel: Some(LocalChannel::new()),
            fut
        }
    }
}

impl<Fut: Future<Output = ()>> Future for LocalChannelUpdater<Fut> {
    type Output = ();

    fn poll(self: PinMut<Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        MY_CHANNEL.with(|my_channel| {
            // TODO: (B) Check this unsafe is actually safe and comment here on why
            unsafe {
                let this = PinMut::get_mut_unchecked(self);
                my_channel.replace(this.channel.take());
                let res = PinMut::new_unchecked(&mut this.fut).poll(cx);
                this.channel = my_channel.replace(None);
                res
            }
        })
    }
}

pub fn spawn<Fut>(fut: Fut) -> impl Future<Output = Result<(), SpawnError>>
where
    Fut: Future<Output = ()> + Send + 'static
{
    let task = LocalChannelUpdater::new(fut);
    future::lazy(move |cx| cx.executor().spawn(task))
}

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] is a bare minimum, and it's recommended
// to include a struct name and maybe even version field in the serialized
// data.
pub trait Message: 'static + Send + Serialize + for<'de> Deserialize<'de> {}

pub struct Pid {
    // TODO: (A) Cross-process / over-the-network messages
    actor_id: ActorId,
}

impl Pid {
    pub fn me() -> Pid {
        Pid {
            actor_id: MY_CHANNEL.with(|c| c.borrow().as_ref().unwrap().actor_id),
        }
    }

    async fn send<M: Message>(&self, msg: Box<M>) -> Result<(), SendError> {
        // TODO: (C) Check these `.unwrap()` are actually sane
        let mut sender = LOCAL_SENDERS.read().unwrap().get(self.actor_id).unwrap();
        await!(sender.send(msg as LocalMessage))
    }
}

pub enum Void {}

// Do not rely on this function being stable. Despite being `pub`, it is part of
// the *internal* API of Erlust, that is to be used by the documented `receive`
// only.
/*
#[doc(hidden)]
pub fn __receive<IgnoreFn, Fut, E>(ignore: IgnoreFn) -> impl Future<Item = LocalMessage, Error = ()>
where
    Fut: Future<Item = bool, Error = ()>,
    IgnoreFn: Fn(&LocalMessage) -> Fut,
{
    MY_CHANNEL.with(|c| {
        // First, attempt to find in waiting list
        for m in c.waiting {
            // TODO: (A) Make it await!() when possible
            if !ignore(&m).wait().unwrap_or(true) {
                return future::Either::A(future::ok(m));
            }
        }

        // Push all irrelevant messages to the waiting list
        // TODO: (B) Make this await!() when possible
        future::Either::B(
            c.receiver
                .by_ref()
                .take_while(ignore)
                .fold(&mut c.waiting, |wait, msg| {
                    wait.push_back(msg);
                    future::ok(wait)
                })
                .and_then(|_| {
                    c.receiver
                    .by_ref()
                    .into_future()
                    .map(|(elt, _)| elt.unwrap()) // TODO: (B) Handle end-of-stream?
                    .map_err(|((), _)| ())
                }),
        )
    })
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestMsg {
        foo: String,
    }
    impl Message for TestMsg {}
}
