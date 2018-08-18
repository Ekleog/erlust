use futures::{channel::mpsc::SendError, SinkExt};

use crate::{ActorId, LocalMessage, LocalSender, Message, LOCAL_SENDERS, MY_CHANNEL};

pub enum Pid {
    Local(LocalPid),
    // TODO: (A) Remote(RemotePid),
}

pub struct LocalPid {
    actor_id: ActorId,
    sender:   LocalSender,
}

impl Pid {
    pub fn me() -> Pid {
        let (actor_id, sender) = MY_CHANNEL.with(|c| {
            let cell = c.borrow();
            let chan = cell.as_ref().unwrap();
            (chan.actor_id, chan.sender.clone())
        });
        Pid::Local(LocalPid { actor_id, sender })
    }

    // TODO: (B) SendError should be a custom type, SendError or RemoteSendError
    pub async fn send<M: Message>(&mut self, msg: Box<M>) -> Result<(), SendError> {
        match *self {
            Pid::Local(ref mut l) => await!(l.sender.send((Pid::me(), msg as LocalMessage))),
        }
        /* For use in Remote()
            // TODO: (C) Check these `.unwrap()` are actually sane
            let mut sender = LOCAL_SENDERS.read().unwrap().get(self.actor_id).unwrap();
            await!(sender.send((LocalPid::me(), msg as LocalMessage)))
        */
    }
}
