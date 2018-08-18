use futures::channel::mpsc;
use serde::{Deserialize, Serialize};
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

pub type ActorId = usize;
pub type LocalMessage = Box<Send + 'static>;
pub type ReceivedMessage = (Pid, LocalMessage);

pub type LocalSender = mpsc::Sender<ReceivedMessage>;
pub type LocalReceiver = mpsc::Receiver<ReceivedMessage>;
