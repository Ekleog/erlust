use futures::{SinkExt, TryFutureExt};

use crate::{ActorId, LocalMessage, LocalSender, Message, Theater, MY_CHANNEL};

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
    theater:  Box<dyn Theater>,
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

    // TODO: (B) SendError should be a custom type, SendError or RemoteSendError
    pub async fn send<M: Message>(&mut self, msg: Box<M>) -> Result<(), failure::Error> {
        match self.0 {
            PidImpl::Local(ref mut l) => await!(l.sender.send((Pid::me(), msg as LocalMessage)).map_err(|e| e.into())),
            PidImpl::Remote(ref mut r) => await!(r.theater.send(my_actor_id() /* , msg */)),
        }
        /* TODO: (A) handle receiving side (in erlust_derive?)
            // TODO: (C) Check these `.unwrap()` are actually sane
            let mut sender = LOCAL_SENDERS.read().unwrap().get(self.actor_id).unwrap();
            await!(sender.send((LocalPid::me(), msg as LocalMessage)))
        */
    }
}
