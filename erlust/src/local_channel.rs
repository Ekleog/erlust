use futures::channel::mpsc;
use std::{cell::RefCell, collections::VecDeque};

use crate::{ActorId, LocalMessage, LocalReceiver, LocalSender, LOCAL_SENDERS};

const QUEUE_BUFFER: usize = 64;
// TODO: (C) make QUEUE_BUFFER configurable
// TODO: (B) limit waiting queue size too

pub struct LocalChannel {
    pub actor_id: ActorId,
    pub sender:   LocalSender,
    pub receiver: LocalReceiver,
    pub waiting:  VecDeque<LocalMessage>,
}

impl LocalChannel {
    pub fn new() -> LocalChannel {
        let (sender, receiver) = mpsc::channel(QUEUE_BUFFER);
        // TODO: (A) make async (qutex + change in my task_local handler) h:https://github.com/Amanieu/parking_lot/issues/86
        let actor_id = LOCAL_SENDERS.write().unwrap().allocate(sender.clone());
        LocalChannel {
            actor_id,
            sender,
            receiver,
            waiting: VecDeque::new(),
        }
    }
}

thread_local! {
    pub static MY_CHANNEL: RefCell<Option<LocalChannel>> = RefCell::new(None);
}
