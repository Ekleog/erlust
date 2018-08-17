use futures::{channel::mpsc::SendError, SinkExt};

use crate::{ActorId, LocalMessage, LocalSender, Message, LOCAL_SENDERS, MY_CHANNEL};

pub struct Pid {
    // TODO: (A) Cross-process / over-the-network messages
    // TODO: (A) Add sender of the message to each received message
    actor_id: ActorId,
    sender:   Option<LocalSender>,
}

impl Pid {
    pub fn me() -> Pid {
        let (actor_id, sender) = MY_CHANNEL.with(|c| {
            let cell = c.borrow();
            let chan = cell.as_ref().unwrap();
            (chan.actor_id, Some(chan.sender.clone()))
        });
        Pid { actor_id, sender }
    }

    // TODO: (B) SendError should be a custom type, SendError or RemoteSendError
    pub async fn send<M: Message>(&mut self, msg: Box<M>) -> Result<(), SendError> {
        if let Some(ref mut sender) = self.sender {
            await!(sender.send(msg as LocalMessage))
        } else {
            // TODO: (C) Check these `.unwrap()` are actually sane
            let mut sender = LOCAL_SENDERS.read().unwrap().get(self.actor_id).unwrap();
            await!(sender.send(msg as LocalMessage))
        }
    }
}
