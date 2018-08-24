use futures::SinkExt;

use crate::{ActorId, LocalMessage, Pid, RemoteMessage, TheaterBox, LOCAL_SENDERS};

// TODO: (C) this 'static shouldn't be needed, it's in TheaterBox's recursive
// bounds
pub async fn inject(
    from: ActorId,
    to: ActorId,
    tag: String,
    msg: Vec<u8>,
    from_theater: Box<dyn 'static + TheaterBox>,
) {
    let mut sender = LOCAL_SENDERS.read().unwrap().get(to).unwrap();
    await!(sender.send((
        Pid::__remote(from, from_theater),
        Box::new(RemoteMessage { tag, msg }) as LocalMessage
    )));
}
