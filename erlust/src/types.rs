use futures::channel::mpsc;
use serde::{Deserialize, Serialize};

// Warning: the Deserialize implementation should be implemented
// in such a way that it fails if anything looks fishy in the message.
// #[serde(deny_unknown_fields)] is a bare minimum, and it's recommended
// to include a struct name and maybe even version field in the serialized
// data.
pub trait Message: 'static + Send + Serialize + for<'de> Deserialize<'de> {
    fn tag() -> &'static str;
}

pub type ActorId = usize;
pub type LocalMessage = Box<Send + 'static>;
pub type LocalSender = mpsc::Sender<LocalMessage>;
pub type LocalReceiver = mpsc::Receiver<LocalMessage>;
