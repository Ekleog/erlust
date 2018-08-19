use erased_serde::Serialize;
use futures::{channel::mpsc, future::FutureObj};
use serde::Deserialize;
use std::any::Any;

use crate::Pid;

pub type ActorId = usize;

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] (at least) is thus recommended
pub trait Message: 'static + Any + Send + Serialize + for<'de> Deserialize<'de> {
    fn tag() -> &'static str;
}

pub trait MessageBox: 'static + Any + Send + Serialize {}

impl<T: Message> MessageBox for T {}

pub type LocalMessage = Box<Send + 'static>; // TODO: (A) make MessageBox h:https://github.com/rust-lang-nursery/futures-rs/issues/1199
pub type ReceivedMessage = (Pid, LocalMessage);

pub struct RemoteMessage(pub Vec<u8>);

pub type LocalSender = mpsc::Sender<ReceivedMessage>;
pub type LocalReceiver = mpsc::Receiver<ReceivedMessage>;

// TODO: (A) provide an implementation of Theater
pub trait Theater: Message {
    // TODO: (B) return impl Trait h:impl-trait-in-trait
    fn send(
        &mut self,
        actor_id: ActorId,
        msg: Vec<u8>, // TODO: (B) think of a way to allow Theater to specify (de)serialization
    ) -> FutureObj<Result<(), failure::Error>>;
}

pub trait TheaterBox: MessageBox {
    fn send(&mut self, actor_id: ActorId, msg: Vec<u8>) -> FutureObj<Result<(), failure::Error>>;
}

impl<T: Theater> TheaterBox for T {
    fn send(&mut self, actor_id: ActorId, msg: Vec<u8>) -> FutureObj<Result<(), failure::Error>> {
        <Self as Theater>::send(self, actor_id, msg)
    }
}
