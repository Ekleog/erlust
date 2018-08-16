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

use futures::{channel::mpsc, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, LinkedList},
    mem::{self, PinMut},
    sync::RwLock,
};

pub use futures::{channel::mpsc::SendError, task::SpawnError};

const QUEUE_BUFFER: usize = 64; // TODO: (C) fiddle
                                // TODO: (B) Limit waiting queue size too

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
        // TODO: (A) make async (qutex + change in my task_local handler)
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
    fut:     Fut,
}

impl<Fut: Future<Output = ()>> LocalChannelUpdater<Fut> {
    fn new(fut: Fut) -> LocalChannelUpdater<Fut> {
        LocalChannelUpdater {
            channel: Some(LocalChannel::new()),
            fut,
        }
    }
}

impl<Fut: Future<Output = ()>> Future for LocalChannelUpdater<Fut> {
    type Output = ();

    fn poll(self: PinMut<Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        MY_CHANNEL.with(|my_channel| {
            // TODO: (B) Check this unsafe is actually safe and comment here on why
            // TODO: (B) Use scoped-tls?
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
    actor_id: ActorId,
}

impl Pid {
    pub fn me() -> Pid {
        Pid {
            actor_id: MY_CHANNEL.with(|c| c.borrow().as_ref().unwrap().actor_id),
        }
    }

    pub async fn send<M: Message>(&self, msg: Box<M>) -> Result<(), SendError> {
        // TODO: (C) Check these `.unwrap()` are actually sane
        let mut sender = LOCAL_SENDERS.read().unwrap().get(self.actor_id).unwrap();
        await!(sender.send(msg as LocalMessage))
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
    // TODO: (B) Do this running-through-the-list in-place
    let waitlist = mem::replace(&mut chan.waiting, LinkedList::new());
    for msg in waitlist {
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

    // TODO: (B) Make this a proper test once https://github.com/rust-lang/rust/issues/53259 solved
    fn check_compiles() {
        // TODO: (A) Add receive! / receive_box! tests
    }
}
