use erased_serde::Serializer;
use futures::future::FutureObj;

use crate::types::{ActorId, Message, MessageBox};

// TODO: (A) provide an implementation of Theater
pub trait Theater: Message {
    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer>;

    // TODO: (B) return impl Trait h:impl-trait-in-trait
    fn send(
        &mut self,
        actor_id: ActorId,
        msg: Vec<u8>, // TODO: (B) think of a way to allow Theater to specify (de)serialization
    ) -> FutureObj<Result<(), failure::Error>>;
}

pub trait TheaterBox: MessageBox {
    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer>;

    fn send(&mut self, actor_id: ActorId, msg: Vec<u8>) -> FutureObj<Result<(), failure::Error>>;
}

impl<T: Theater> TheaterBox for T {
    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer> {
        <Self as Theater>::serializer(self, out)
    }

    fn send(&mut self, actor_id: ActorId, msg: Vec<u8>) -> FutureObj<Result<(), failure::Error>> {
        <Self as Theater>::send(self, actor_id, msg)
    }
}
