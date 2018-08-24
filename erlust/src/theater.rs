use erased_serde::{Deserializer, Serializer};
use futures::future::FutureObj;

use crate::types::{ActorId, Message, MessageBox};

// TODO: (A) provide an implementation of Theater
pub trait Theater: Message + Clone {
    fn here(&mut self) -> Box<Self>;

    // A TheaterBox that can be used by the one on the other side of this Theater
    // to contact the TheaterBox o that can be contacted locally
    fn sees_as(&mut self, o: Box<dyn TheaterBox>) -> Box<Self>;

    // TODO: (B) return associated type
    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer>;
    fn deserializer<'de>(&mut self, inp: &Vec<u8>) -> Box<Deserializer<'de>>;

    // TODO: (B) return impl Trait h:impl-trait-in-trait
    fn send(
        &mut self,
        from: ActorId,
        to: ActorId,
        tag: &'static str,
        msg: Vec<u8>,
    ) -> FutureObj<Result<(), failure::Error>>;
}

pub trait TheaterBox: MessageBox {
    fn here(&mut self) -> Box<dyn TheaterBox>;

    fn clone_to_box(&self) -> Box<dyn TheaterBox>;

    fn sees_as(&mut self, o: Box<dyn TheaterBox>) -> Box<dyn TheaterBox>;

    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer>;
    fn deserializer<'de>(&mut self, inp: &'de Vec<u8>) -> Box<Deserializer<'de>>;

    fn send(
        &mut self,
        from: ActorId,
        to: ActorId,
        tag: &'static str,
        msg: Vec<u8>,
    ) -> FutureObj<Result<(), failure::Error>>;
}

impl<T: Theater> TheaterBox for T {
    fn here(&mut self) -> Box<TheaterBox> {
        <Self as Theater>::here(self)
    }

    fn clone_to_box(&self) -> Box<dyn TheaterBox> {
        Box::new(<Self as Clone>::clone(self))
    }

    fn sees_as(&mut self, o: Box<dyn TheaterBox>) -> Box<dyn TheaterBox> {
        <Self as Theater>::sees_as(self, o)
    }

    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer> {
        <Self as Theater>::serializer(self, out)
    }

    fn deserializer<'de>(&mut self, inp: &'de Vec<u8>) -> Box<Deserializer<'de>> {
        <Self as Theater>::deserializer(self, inp)
    }

    fn send(
        &mut self,
        from: ActorId,
        to: ActorId,
        tag: &'static str,
        msg: Vec<u8>,
    ) -> FutureObj<Result<(), failure::Error>> {
        <Self as Theater>::send(self, from, to, tag, msg)
    }
}

serialize_trait_object!(TheaterBox);
