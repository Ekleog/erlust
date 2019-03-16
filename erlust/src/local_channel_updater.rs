use futures::{task, Future, Poll};
use std::pin::Pin;

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

    fn poll(self: Pin<&mut Self>, lw: &task::Waker) -> Poll<Self::Output> {
        MY_CHANNEL.with(|my_channel| {
            // TODO: (B) Check this unsafe is actually safe and comment here on why
            // TODO: (B) Use scoped-tls?
            unsafe {
                let this = Pin::get_unchecked_mut(self);
                my_channel.replace(this.channel.take());
                let res = Pin::new_unchecked(&mut this.fut).poll(lw);
                this.channel = my_channel.replace(None);
                res
            }
        })
    }
}
