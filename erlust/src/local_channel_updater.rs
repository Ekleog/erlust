use futures::{task, Future, Poll};
use std::mem::PinMut;

use crate::{LocalChannel, MY_CHANNEL};

pub struct LocalChannelUpdater<Fut: Future<Output = ()>> {
    channel: Option<LocalChannel>,
    fut:     Fut,
}

impl<Fut: Future<Output = ()>> LocalChannelUpdater<Fut> {
    pub fn new(fut: Fut) -> LocalChannelUpdater<Fut> {
        LocalChannelUpdater {
            channel: Some(LocalChannel::new()),
            fut,
        }
    }
}

impl<Fut: Future<Output = ()>> Future for LocalChannelUpdater<Fut> {
    type Output = ();

    fn poll(self: PinMut<Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        MY_CHANNEL.with(|my_channel| {
            // TODO: (B) Check this unsafe is actually safe and comment here on why
            // TODO: (B) Use scoped-tls?
            unsafe {
                let this = PinMut::get_mut_unchecked(self);
                my_channel.replace(this.channel.take());
                let res = PinMut::new_unchecked(&mut this.fut).poll(cx);
                this.channel = my_channel.replace(None);
                res
            }
        })
    }
}
