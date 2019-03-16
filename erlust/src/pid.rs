//! Ways to transparently send messages to actors both locally and remotely

use erased_serde::Serialize as ErasedSerdeSerialize;
use futures::{SinkExt, TryFutureExt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ActorId, LocalSender, Message, ReceivedMessage, TheaterBox, HERE, MY_CHANNEL};

/// The address of an actor, used to send it messages
pub struct Pid(PidImpl);

/// Actual implementation of a [`Pid`] (used because `enum`s cannot be private)
enum PidImpl {
    /// [`Pid`] from the local theater
    Local(LocalPid),

    /// [`Pid`] for an actor located in a remote [`Theater`] (that can
    /// potentially be the local theater too, but it will go through the
    /// whole [`Theater`] handling then)
    Remote(RemotePid),
}

/// A [`Pid`] for an actor in the local theater
struct LocalPid {
    /// The actor's identifier key for if the actor is to be converted into a
    /// remote actor
    actor_id: ActorId,

    /// The local sender for local usage
    sender: LocalSender,
}

/// A [`Pid`] for an actor in a potentially remote theater
#[derive(Deserialize, Serialize)]
struct RemotePid {
    /// The actor's identifier in the remote theater
    actor_id: ActorId,

    /// The remote theater in which the actor is located
    theater: Box<dyn TheaterBox>,
}

/// Helper to get the [`ActorId`] for the currently-running actor
///
/// Panics if not called from an actor task.
fn my_actor_id() -> ActorId {
    MY_CHANNEL.with(|c| c.borrow().as_ref().unwrap().actor_id)
}

impl Pid {
    /// Returns the address of the actor currently running
    ///
    /// Panics if not called from an actor task.
    pub fn me() -> Pid {
        let (actor_id, sender) = MY_CHANNEL.with(|c| {
            let cell = c.borrow();
            let chan = cell.as_ref().unwrap();
            (chan.actor_id, chan.sender.clone())
        });
        Pid(PidImpl::Local(LocalPid { actor_id, sender }))
    }

    /// Builder for a remote actor from its raw parts
    #[doc(hidden)]
    pub fn __remote(actor_id: ActorId, theater: Box<dyn TheaterBox>) -> Pid {
        Pid(PidImpl::Remote(RemotePid { actor_id, theater }))
    }

    /// Gets the theater in which this [`Pid`] is located
    ///
    /// Panics if `self` is not a [`RemotePid`].
    #[doc(hidden)]
    pub fn __theater_assert_remote(&self) -> Box<dyn TheaterBox> {
        if let PidImpl::Remote(ref r) = self.0 {
            r.theater.clone_to_box()
        } else {
            unreachable!()
        }
    }

    /// Sends `msg` to `self`
    ///
    /// Fails if the message could not be sent. Please remember that depending
    /// on the [`Theater`] implementation, some messages may still be lost
    /// in transit, even if this function did not return `Err` to the
    /// caller.
    // TODO: (B) either replace failure::Error by a better type or document why not
    pub async fn send<M: Message>(&mut self, msg: Box<M>) -> Result<(), failure::Error> {
        match self.0 {
            PidImpl::Local(ref mut l) => await!(l
                .sender
                .send(ReceivedMessage::Local((Pid::me(), msg)))
                .map_err(|e| e.into())),
            PidImpl::Remote(ref mut r) => {
                // TODO: (B) have the theater-provided serializer asyncly send on-the-fly?
                // Note: if erased_serialize can yield, will have to replace the thread_local
                // usage with a task_local one.
                let mut vec = Vec::with_capacity(128);
                let mut erased_ser = r.theater.serializer(&mut vec);
                HERE.with(|here| -> Result<(), erased_serde::Error> {
                    *here.borrow_mut() = Some(r.theater.here());
                    msg.erased_serialize(&mut erased_ser)?;
                    *here.borrow_mut() = None;
                    Ok(())
                })?;
                await!(r.theater.send(my_actor_id(), r.actor_id, M::tag(), vec))
            }
        }
    }
}

impl Serialize for Pid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut here = HERE.with(|h| h.borrow().as_ref().unwrap().clone_to_box());
        match self.0 {
            PidImpl::Local(ref l) => {
                let seen_from_remote = RemotePid {
                    actor_id: l.actor_id,
                    theater:  here,
                };
                seen_from_remote.serialize(serializer)
            }
            PidImpl::Remote(ref r) => {
                let seen_from_remote = RemotePid {
                    actor_id: r.actor_id,
                    theater:  here.sees_as(r.theater.clone_to_box()),
                };
                seen_from_remote.serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for Pid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let r = RemotePid::deserialize(deserializer)?;
        Ok(Pid(PidImpl::Remote(r)))
    }
}
