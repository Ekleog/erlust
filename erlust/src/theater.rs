//! Protocol for sending messages to remote actors

use erased_serde::{Deserializer, Serializer};
use futures::future::FutureObj;
use serde::de::Error as SerdeDeError;
use std::cell::RefCell;

use crate::types::{ActorId, Message, MessageBox};

/// A bunch of actors and functions used for theaters to communicate between
/// them
// TODO: (A) provide an implementation of Theater
pub trait Theater: Message + Clone {
    /// Returns the local theater, as seen from the theater defined by `self`
    ///
    /// For instance, if the link between the local theater and `self` is a
    /// named pipe, then `here` should return a theater that points to the
    /// part of the pipe that should be used by the remote end to connect
    /// to the local end.
    ///
    /// If the link between the local theater and `self` is
    /// TCP-on-a-flat-network, then this function should return a
    /// theater that points to the address and port on which the local
    /// process is currently listening.
    fn here(&mut self) -> Box<Self>;

    /// Returns an instance of `Self` that can be used by the remote end of
    /// this connection (ie. `self`) to communicate with `other` (which is
    /// defined by a [`TheaterBox`] reachable from the current theater)
    ///
    /// For instance, for a TCP-on-a-flat-network connection model, this would
    /// simply be a no-op (only downcasting `other` into an instance of the
    /// local theater).
    ///
    /// However, things get fun when handling communicating protocols, like a
    /// protocol for making multiple processes on the same machine and
    /// another protocol for making multiple machines communicate, each
    /// optimized to handle only their use case. In this case (ie. if `self`
    /// cannot actually directly talk with `other`), it may become necessary to
    /// setup a “bouncer” actor in the local theater, and then return a
    /// pointer to said local actor, which would then be charged with
    /// relaying messages to `other`
    // TODO: (A) this should take an actor and return an actor (cf. doc above)
    fn sees_as(&mut self, other: Box<dyn TheaterBox>) -> Box<Self>;

    /// Returns a serializer to be used for generating the `msg` argument of
    /// [`send`] out of a [`Message`]
    ///
    /// `out` will be passed to [`send`] after serialization of said message.
    // TODO: (B) return associated type?
    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer>;

    /// Returns a deserializer to be used for deserializing the `msg` argument
    /// from [`inject`]
    ///
    /// Usually, this will be the operation opposite to the one [`serializer`]
    /// performed.
    // TODO: (B) return associated type?
    fn deserializer<'de>(&mut self, inp: &'de Vec<u8>) -> Box<Deserializer<'de>>;

    /// Send a message to `self`
    ///
    /// This should trigger a call to [`inject`] in the theater designed by
    /// `self`,  with the same arguments as the ones passed to this
    /// function, plus a `from_theater` equivalent to `self.here()`.
    ///
    /// Please note however that the security model assumes that the
    /// *receiving* side identifies the `from_theater`, not the *sending* side,
    /// so serializing `self.here()` would most likely be a bad idea.
    // TODO: (B) return impl Trait h:impl-trait-in-trait
    // TODO: (A) make `tag` a `String`
    fn send(
        &mut self,
        from: ActorId,
        to: ActorId,
        tag: &'static str,
        msg: Vec<u8>,
    ) -> FutureObj<Result<(), failure::Error>>;
}

/// A [`Box`]-able [`Theater`]
///
/// This trait is automatically implemented for all traits implementing
/// [`Theater`].
pub trait TheaterBox: MessageBox {
    /// See [`Theater::here`]
    fn here(&mut self) -> Box<dyn TheaterBox>;

    /// Clones `self` into a [`Box`]
    fn clone_to_box(&self) -> Box<dyn TheaterBox>;

    /// Deserializes from `inp` into the type of `Self`, in a type-erased way
    /// (ie. into a trait object)
    fn deserialize_as_self(
        &self,
        inp: &mut Deserializer,
    ) -> Result<Box<TheaterBox>, erased_serde::Error>;

    /// See [`Theater::sees_as`]
    fn sees_as(&mut self, other: Box<dyn TheaterBox>) -> Box<dyn TheaterBox>;

    /// See [`Theater::serializer`]
    fn serializer(&mut self, out: &mut Vec<u8>) -> Box<Serializer>;

    /// See [`Theater::deserializer`]
    fn deserializer<'de>(&mut self, inp: &'de Vec<u8>) -> Box<Deserializer<'de>>;

    /// See [`Theater::send`]
    fn send(
        &mut self,
        from: ActorId,
        to: ActorId,
        tag: &'static str,
        msg: Vec<u8>,
    ) -> FutureObj<Result<(), failure::Error>>;
}

// TODO: (B) use scoped_tls
thread_local! {
    /// The local theater, defined in relation to the remote theater currently being used
    ///
    /// This is designed to handle correctly cases of communicating protocols with a gateway that handles multiple
    /// protocols, and thus has a different address on each protocol.
    pub static HERE: RefCell<Option<Box<TheaterBox>>> = RefCell::new(None);
}

impl<T: Theater> TheaterBox for T {
    fn here(&mut self) -> Box<TheaterBox> {
        <Self as Theater>::here(self)
    }

    fn clone_to_box(&self) -> Box<dyn TheaterBox> {
        Box::new(<Self as Clone>::clone(self))
    }

    fn deserialize_as_self(
        &self,
        inp: &mut Deserializer,
    ) -> Result<Box<TheaterBox>, erased_serde::Error> {
        erased_serde::deserialize::<Box<Self>>(inp).map(|t| t as Box<TheaterBox>)
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

impl<'de> serde::Deserialize<'de> for Box<dyn TheaterBox> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut d = erased_serde::Deserializer::erase(deserializer);
        HERE.with(|h| {
            h.borrow()
                .as_ref()
                .unwrap()
                .deserialize_as_self(&mut d)
                .map_err(|e| D::Error::custom(format!("{}", e)))
            // TODO: (B) try to proxy full error
        })
    }
}
