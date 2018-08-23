use futures::SinkExt;

use crate::{ActorId, LocalMessage, Pid, RemoteMessage, TheaterBox, LOCAL_SENDERS};

// TODO: (C) this 'static shouldn't be needed, it's in TheaterBox's recursive
// bounds
pub async fn inject(
    to: ActorId,
    msg: Vec<u8>,
    from_theater: Box<dyn 'static + TheaterBox>,
    from: ActorId, // TODO: (A) serialize and deserialize this, can't be given by transport
) {
    let mut sender = LOCAL_SENDERS.read().unwrap().get(to).unwrap();
    await!(sender.send((
        Pid::__remote(from, from_theater),
        Box::new(RemoteMessage(msg)) as LocalMessage
    )));
}
