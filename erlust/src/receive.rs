use futures::{Future, StreamExt};
use std::mem;

use crate::{LocalMessage, Pid, ReceivedMessage, MY_CHANNEL};

pub enum ReceiveResult<Ret> {
    Use(Ret),
    Skip(ReceivedMessage),
}

// Note: it's highly recommended to use by setting a timeout on the returned
// future
pub async fn receive<HandleFn, Fut, Ret>(handle: HandleFn) -> Ret
where
    Fut: Future<Output = ReceiveResult<Ret>>,
    HandleFn: Fn(ReceivedMessage) -> Fut,
{
    use self::ReceiveResult::*;

    // This `expect` shouldn't trigger, because `LocalChannelUpdater` should always
    // keep `MY_CHANNEL` task-local. As such, the only moment where it should be
    // set to `None` is here, and it is restored before the end of this function,
    // and `__receive` cannot be called inside `__receive`.
    let mut chan = MY_CHANNEL
        .with(|c| c.borrow_mut().take())
        .expect("Called receive inside receive");

    // First, attempt to find a message in waiting list
    // Use a temporary variable to work around https://github.com/rust-lang/rust/issues/59245
    let l = chan.waiting.len();
    for i in 0..l {
        // TODO: (C) consider unsafe here to remove the temp. var., dep. on benchmarks
        let mut msg = ReceivedMessage::Local((Pid::me(), Box::new(()) as LocalMessage));
        mem::swap(&mut msg, &mut chan.waiting[i]);
        match handle(msg).await {
            Use(ret) => {
                chan.waiting.remove(i);
                MY_CHANNEL.with(|c| *c.borrow_mut() = Some(chan));
                return ret;
            }
            Skip(msg) => {
                chan.waiting[i] = msg;
            }
        }
    }

    // Push all irrelevant messages to the waiting list, then return relevant one
    loop {
        // This `expect` shouldn't trigger, because `chan.receiver.next()` is
        // supposed to answer `None` iff all `Sender`s associated to the channel
        // have been dropped. Except we always keep a `Sender` alive in the
        // `LOCAL_SENDERS` map, and `__receive` should not be able to be called
        // once the actor has been dropped, so this should be safe.
        let msg = chan
            .receiver
            .next()
            .await
            .expect("Called receive after the actor was dropped");
        match handle(msg).await {
            Use(ret) => {
                MY_CHANNEL.with(|c| *c.borrow_mut() = Some(chan));
                return ret;
            }
            Skip(msg) => {
                chan.waiting.push_back(msg);
            }
        }
    }
}
