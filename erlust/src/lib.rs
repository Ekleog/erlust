#![feature(
    arbitrary_self_types,
    async_await,
    await_macro,
    futures_api,
    pin
)]

#[macro_use]
extern crate erased_serde;
extern crate failure;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod inject;
mod local_channel;
mod local_channel_updater;
mod local_senders;
mod pid;
mod receive;
mod spawn;
mod theater;
mod types;

use self::{
    local_channel::{LocalChannel, MY_CHANNEL},
    local_channel_updater::LocalChannelUpdater,
    local_senders::LOCAL_SENDERS,
    theater::{TheaterBox, HERE},
    types::{ActorId, LocalMessage, LocalReceiver, LocalSender, ReceivedMessage},
};

pub use futures::{channel::mpsc::SendError, task::SpawnError};

pub use self::{
    inject::inject,
    pid::Pid,
    receive::{receive, ReceiveResult},
    spawn::spawn,
    theater::Theater,
    types::{Message, RemoteMessage},
};

// TODO: (A) add a local registry to record name<->Pid associations?
// TODO: (B) write a library offering a global registry for name<->Pid

// TODO: (A) document all the things
// TODO: (A) test all the things

// TODO: (A) implement links & monitors
// TODO: (A) implement cross-process links & monitors

// TODO: (B) consider using pub(crate) instead of #[doc(hidden)]
