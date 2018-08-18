use erased_serde::Serialize;
use futures::{channel::mpsc, future::FutureObj};
use serde::Deserialize;
use std::any::Any;

use crate::Pid;

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] is a bare minimum, and it's recommended
// to include a struct name and maybe even version field in the serialized
// data.
//
// Message is not object-safe, thus cannot be used for LocalMessage.
pub trait Message: 'static + Any + Send + Serialize + for<'de> Deserialize<'de> {
    fn tag() -> &'static str;
}

pub trait MessageBox: 'static + Any + Send + Serialize {}

impl<T: Message> MessageBox for T {}

pub type ActorId = usize;
pub type LocalMessage = Box<Send + 'static>; // TODO: (A) make MessageBox
pub type ReceivedMessage = (Pid, LocalMessage);

pub type LocalSender = mpsc::Sender<ReceivedMessage>;
pub type LocalReceiver = mpsc::Receiver<ReceivedMessage>;

pub trait Theater: Message {
    // TODO: (B) remove Box h:impl-trait-in-trait
    fn send(
        &mut self,
        actor_id: ActorId, // , msg: Message
    ) -> FutureObj<Result<(), failure::Error>>;
}

pub trait TheaterBox: MessageBox {
    fn send(
        &mut self,
        actor_id: ActorId, // , msg: Message
    ) -> FutureObj<Result<(), failure::Error>>;
}

impl<T: Theater> TheaterBox for T {
    fn send(
        &mut self,
        actor_id: ActorId, // , msg: Message
    ) -> FutureObj<Result<(), failure::Error>> {
        <Self as Theater>::send(self, actor_id)
    }
}
