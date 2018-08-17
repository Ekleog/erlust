use futures::{
    future,
    task::{SpawnError, SpawnExt},
    Future,
};

use crate::LocalChannelUpdater;

pub fn spawn<Fut>(fut: Fut) -> impl Future<Output = Result<(), SpawnError>>
where
    Fut: Future<Output = ()> + Send + 'static,
{
    let task = LocalChannelUpdater::new(fut);
    future::lazy(move |cx| cx.spawner().spawn(task))
}
