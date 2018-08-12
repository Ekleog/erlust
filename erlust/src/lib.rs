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

    pub async fn send<M: Message>(&self, msg: Box<M>) -> Result<(), SendError> {
        // TODO: (C) Check these `.unwrap()` are actually sane
        let mut sender = LOCAL_SENDERS.read().unwrap().get(self.actor_id).unwrap();
        await!(sender.send(msg as LocalMessage))
    }
}

#[doc(hidden)]
pub async fn __receive<WantFn, Fut>(want: WantFn) -> LocalMessage
where
    Fut: Future<Output = bool>,
    WantFn: Fn(&LocalMessage) -> Fut,
{
    // This `expect` shouldn't trigger, because `LocalChannelUpdater` should always
    // keep `MY_CHANNEL` task-local. As such, the only moment where it should be
    // set to `None` is here, and it is restored before the end of this function,
    // and `__receive` cannot be called inside `__receive`.
    let mut chan = MY_CHANNEL
        .with(|c| c.borrow_mut().take())
        .expect("Called receive inside receive");;

    // First, attempt to find a message in waiting list
    let waitlist = mem::replace(&mut chan.waiting, LinkedList::new());
    for msg in waitlist {
        if await!(want(&msg)) {
            MY_CHANNEL.with(|c| *c.borrow_mut() = Some(chan));
            return msg;
        } else {
            chan.waiting.push_back(msg);
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
        if await!(want(&msg)) {
            MY_CHANNEL.with(|c| *c.borrow_mut() = Some(chan));
            return msg;
        }
        chan.waiting.push_back(msg);
    }
}

// TODO: (A) add tag() for trait Message, custom_derive to fill it in from attribute
// (but do not default to the struct type)

#[doc(hidden)]
#[macro_export]
macro_rules! erlust_util {
    // @do_receive
    ( @do_receive $boxed:tt $($x:tt)+ ) => {
        erlust_util!(@do_match $is_box to_exec
            (
                $crate::__receive(|msg: &LocalMessage| {
                    erlust_util!(@do_match $boxed to_bool (*msg) $($x:tt)+)
                })
            )
            $($x:tt)*
        )
    };

    // @do_match
    (
        @do_match $boxed:tt $to:tt ( $var:expr )
        $typ:ty : $pattern:pat $(if $guard:expr)* => $body:expr ,
        $($next:tt)*
    ) => {
        erlust_util!(
            @do_match $boxed $to ($var)
            $typ : $pattern $(if $guard)* => { $body }
            $($next)*
        )
    };

    (
        @do_match $boxed:tt to_bool ( $var:expr )
        $typ:ty : $pattern:pat $(if $guard:expr)* => $body:block $(,)*
        $($next:tt)*
    ) => {
        match (&*$var).downcast_ref::<$typ>() {
            Some($pattern) $(if $guard)* => true,
            None => erlust_util!(@do_match to_bool ($var) $($next)*)
        }
    };

    ( @do_match $boxed:tt to_bool ( $var:expr ) ) => {
        false
    };

    (
        @do_match $boxed:tt to_expr ( $var:expr )
        $typ:ty : $pattern:pat $(if $guard:expr)* => $body:block $(,)*
        $($next:tt)*
    ) => {
        match $var.downcast::<$typ>() {
            Ok(res) if {
                if let $pattern = &*res {
                    $($guard)#* // The # should (hopefully) trigger a syntax error
                } else {
                    false
                }
            } => {
                erlust_util!(@exec_body $boxed ( res ) $pattern $body)
            },
            Err(b) => erlust_util!(@do_match to_expr ($var) $($next)*),
        }
    };

    // TODO: (C) consider making this unreachable_unchecked (needs benchmark)
    ( @do_match $boxed:tt to_expr ( $var:expr ) ) => {
        unreachable!()
    };

    // @exec_body
    ( @exec_body unboxed ( $var:expr ) $pattern:pat $body:block ) => {
        {
            let $pattern = *$var;
            $body
        }
    }

    ( @exec_body boxed ( $var:expr ) $pattern:pat $body:block ) => {
        {
            let $pattern = $var;
            $body
        }
    }
}

// Being given:
//
//  receive! {
//      (usize, String): (1, y) if baz(y) => quux(y),
//      usize: x if foo(x) => bar(x),
//  }
//
// With types:
//  * `baz`:  `Fn(&String) -> bool`
//  * `quux`: `Fn(String) -> T`
//  * `foo`:  `Fn(&usize) -> bool`
//  * `bar`:  `Fn(usize) -> T`
//
// Expands to:
//
//  match __receive(|msg: &LocalMessage| {
//      match (&**msg).downcast_ref::<(usize, String)>() {
//          Some((1, y)) if baz(y) => true,
//          None => match (&**msg).downcast_ref::<usize>() {
//              Some(x) if foo(x) => true,
//              None => false,
//          }
//      }
//  }).downcast::<(usize, String)>() {
//      Ok(res) if { if let (1, y) = &*res { baz(y) } else { false } } => quux(y),
//      Err(b) => match b.downcast::<usize>() {
//          Ok(res) if { if let x = &*res { foo(x) } else { false } } => bar(x),
//          Err(_) => unreachable!(),
//      }
//  }

#[macro_export]
macro_rules! receive {
    ( $($x:tt)+ ) => {
        erlust_util!(@do_receive unboxed $($x)+)
    };
}

// Being given:
//
//  receive_box! {
//      Box<(usize, String)>: (1, y) if baz(y) => quux(y),
//      Box<usize>: x if foo(x) => bar(x),
//  }
//
// With types:
//  * `baz`:  `Fn(&String) -> bool`
//  * `quux`: `Fn(Box<String>) -> T`
//  * `foo`:  `Fn(&usize) -> bool`
//  * `bar`:  `Fn(Box<usize>) -> T`
//
// Expands to:
//
//  match __receive(|msg: &LocalMessage| {
//      match (&**msg).downcast_ref::<(usize, String)>() {
//          Some((1, y)) if baz(y) => true,
//          None => match (&**msg).downcast_ref::<usize>() {
//              Some(x) if foo(x) => true,
//              None => false,
//          }
//      }
//  }).downcast::<(usize, String)>() {
//      Ok(res) if { if let (1, y) = &*res { baz(y) } else { false } } => quux(y),
//      Err(b) => match b.downcast::<usize>() {
//          Ok(res) if { if let x = &*res { foo(x) } else { false } } => bar(x),
//          Err(_) => unreachable!(),
//      }
//  }

#[macro_export]
macro_rules! receive_box {
    ( $($x:tt)+ ) => {
        erlust_util!(@do_receive boxed $($x)+)
    }
}

// TODO: (A) match just refuses to bind by-move in guard, this'd makes things simpler for us
// TODO: (B) Make semantics of receive! / receive_box! precise wrt. borrowing and owning
// TODO: (B) Make sure receive! / receive_box! accept exactly the same syntax as match

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestMsg {
        foo: String,
    }
    impl Message for TestMsg {}

    // TODO: (B) Make this a proper test once https://github.com/rust-lang/rust/issues/53259 solved
    fn check_compiles() {
        // TODO: (A) Add receive! / receive_box! tests
    }
}
