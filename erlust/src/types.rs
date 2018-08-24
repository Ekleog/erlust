use futures::channel::mpsc;
use serde::Deserialize;
use std::any::Any;

use crate::Pid;

pub type ActorId = usize;

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] (at least) is thus recommended
pub trait Message: 'static + Any + Send + serde::Serialize + for<'de> Deserialize<'de> {
    fn tag() -> &'static str;
}

pub trait MessageBox: 'static + Any + Send + erased_serde::Serialize {}

impl<T: Message> MessageBox for T {}

pub type LocalMessage = Box<Send + 'static>; // TODO: (A) make MessageBox h:https://github.com/rust-lang-nursery/futures-rs/issues/1199
pub type ReceivedMessage = (Pid, LocalMessage);

pub struct RemoteMessage {
    pub tag: String,
    pub msg: Vec<u8>,
}

pub type LocalSender = mpsc::Sender<ReceivedMessage>;
pub type LocalReceiver = mpsc::Receiver<ReceivedMessage>;
