use futures::{
    task::{Spawn, SpawnError, SpawnExt},
    Future,
};

use crate::LocalChannelUpdater;

// TODO(A): figure out a way to not require this `spawner` argument
// TODO(B): consider making the output impl Future again as future-proofing
pub fn spawn<Spwn, Fut>(spawner: &mut Spwn, fut: Fut) -> Result<(), SpawnError>
where
    Spwn: Spawn,
    Fut: Future<Output = ()> + Send + 'static,
{
    let task = LocalChannelUpdater::new(fut);
    spawner.spawn(task)
}
