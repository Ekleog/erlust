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
    theater::TheaterBox,
    types::{ActorId, LocalMessage, LocalReceiver, LocalSender, ReceivedMessage, RemoteMessage},
};

pub use futures::{channel::mpsc::SendError, task::SpawnError};

pub use self::{
    inject::inject,
    pid::Pid,
    receive::{receive, ReceiveResult},
    spawn::spawn,
    theater::Theater,
    types::Message,
};

// TODO: (A) add a registry to record name<->Pid associations?

// TODO: (A) document all the things
// TODO: (A) test all the things
