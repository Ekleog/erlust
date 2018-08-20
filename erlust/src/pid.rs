use futures::{SinkExt, TryFutureExt};

use crate::{ActorId, LocalMessage, LocalSender, Message, TheaterBox, MY_CHANNEL};

// TODO: (A) implement Message for Pid
pub struct Pid(PidImpl);

enum PidImpl {
    Local(LocalPid),
    Remote(RemotePid),
}

struct LocalPid {
    actor_id: ActorId,
    sender:   LocalSender,
}

struct RemotePid {
    actor_id: ActorId,
    theater:  Box<dyn TheaterBox>,
}

fn my_actor_id() -> ActorId {
    MY_CHANNEL.with(|c| c.borrow().as_ref().unwrap().actor_id)
}

impl Pid {
    pub fn me() -> Pid {
        let (actor_id, sender) = MY_CHANNEL.with(|c| {
            let cell = c.borrow();
            let chan = cell.as_ref().unwrap();
            (chan.actor_id, chan.sender.clone())
        });
        Pid(PidImpl::Local(LocalPid { actor_id, sender }))
    }

    #[doc(hidden)]
    pub fn __remote(actor_id: ActorId, theater: Box<dyn TheaterBox>) -> Pid {
        Pid(PidImpl::Remote(RemotePid { actor_id, theater }))
    }

    // TODO: (B) either replace failure::Error by a better type or document why not
    pub async fn send<M: Message>(&mut self, msg: Box<M>) -> Result<(), failure::Error> {
        match self.0 {
            PidImpl::Local(ref mut l) => await!(
                l.sender
                    .send((Pid::me(), msg as LocalMessage))
                    .map_err(|e| e.into())
            ),
            PidImpl::Remote(ref mut r) => {
                // TODO: (B) have the theater-provided serializer asyncly send on-the-fly?
                let mut vec = Vec::with_capacity(128);
                let mut erased_ser = r.theater.serializer(&mut vec);
                msg.erased_serialize(&mut erased_ser)?;
                await!(r.theater.send(my_actor_id(), vec))
            }
        }
    }
}
