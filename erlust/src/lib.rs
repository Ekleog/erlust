#[macro_use]
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

use futures::{future, sync::mpsc, Future, Sink};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::RwLock};

const QUEUE_BUFFER: usize = 64; // TODO: (C) fiddle

type ActorId = usize;
type Sender = mpsc::Sender<Box<Send>>;
type Receiver = mpsc::Receiver<Box<Send>>;

struct Senders {
    next_actor_id: ActorId,
    map: HashMap<ActorId, Sender>,
}

impl Senders {
    fn new() -> Senders {
        Senders {
            next_actor_id: 0,
            map: HashMap::new(),
        }
    }

    fn allocate(&mut self, sender: Sender) -> ActorId {
        let actor_id = self.next_actor_id;
        // TODO: (C) try to handle gracefully the overflow case
        self.next_actor_id = self.next_actor_id.checked_add(1).unwrap();
        self.map.insert(actor_id, sender);
        actor_id
    }

    fn get(&self, actor_id: ActorId) -> Option<Sender> {
        self.map.get(&actor_id).map(|s| s.clone())
    }
}

lazy_static! {
    static ref SENDERS: RwLock<Senders> = RwLock::new(Senders::new());
}

struct LocalChannel {
    actor_id: ActorId,
    receiver: Receiver,
}

impl LocalChannel {
    fn new() -> LocalChannel {
        let (sender, receiver) = mpsc::channel(QUEUE_BUFFER);
        // TODO: (A) make async (qutex + https://github.com/rust-lang-nursery/futures-rs/issues/1187 ?)
        let actor_id = SENDERS.write().unwrap().allocate(sender);
        LocalChannel { actor_id, receiver }
    }
}

task_local! {
    static MY_CHANNEL: LocalChannel = LocalChannel::new()
}

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] is a bare minimum, and it's recommended
// to include a struct name and maybe even version field in the serialized
// data.
pub trait Message: 'static + Send + Serialize + for<'de> Deserialize<'de> {}

pub struct SendError {
    _priv: (),
}

impl SendError {
    fn new() -> SendError {
        SendError { _priv: () }
    }
}

pub struct Pid {
    // TODO: (A) Cross-process / over-the-network messages
    actor_id: ActorId,
}

impl Pid {
    pub fn me() -> Pid {
        Pid {
            actor_id: MY_CHANNEL.with(|c| c.actor_id),
        }
    }

    pub fn send<M: Message>(&self, msg: Box<M>) -> impl Future<Item = (), Error = SendError> {
        // TODO: (C) Check these `.unwrap()` are actually sane
        let sender = SENDERS.read().unwrap().get(self.actor_id).unwrap();
        sender
            .send(msg as Box<Send>)
            .map(|_| ())
            .map_err(|_| SendError::new())
    }
}

pub enum Void {}

pub fn receive(
    _wanted: &Fn(&Send) -> bool,
) -> impl Future<Item = Box<Send + 'static>, Error = Void> {
    future::ok(Box::new(()) as Box<Send>) // TODO: (A) implement
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
