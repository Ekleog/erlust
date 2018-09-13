//! Helper for sending a message for local actors

use futures::SinkExt;

use crate::{ActorId, LocalMessage, Pid, RemoteMessage, TheaterBox, LOCAL_SENDERS};

/// Injects a message from another theater to a local actor
///
/// All the parameters ***except for `from_theater`*** should be inserted as sent by the remote
/// theater, ie. as passed to [`Theater::send`] on the remote actor:
///  * `from` is the sending (remote) [`ActorId`], as reported by the remote theater by theater-specific means
///  * `to` is the receiving (local) [`ActorId`], as requested by the remote theater
///  * `tag` is a tag that identifies the message type
///  * `msg` is the (serialized) message
///
/// `from_theater` ***must not*** be taken as trusted from the remote theater! This would break the
/// security model of erlust, which is based on the fact that theaters don't necessarily trust
/// other theaters (but actors within a theater trust each other).
///
/// As a consequence, `from_theater` ***must*** be computed locally based on the way the message
/// has been received. For instance, if it came from a TLS connection, `from_theater` can be
/// inferred from the connection parameters to identify the theater on the other side.
// TODO: (C) this 'static shouldn't be needed, it's in TheaterBox's recursive
// bounds
pub async fn inject(
    from: ActorId,
    to: ActorId,
    tag: String,
    msg: Vec<u8>,
    from_theater: Box<dyn 'static + TheaterBox>,
) {
    // TODO: (A) do not panic if the local sender doesn't exist
    let mut sender = LOCAL_SENDERS.read().unwrap().get(to).unwrap();
    await!(sender.send((
        Pid::__remote(from, from_theater),
        Box::new(RemoteMessage { tag, msg }) as LocalMessage
    )));
}
