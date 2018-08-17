use futures::channel::mpsc;

pub type ActorId = usize;
pub type LocalMessage = Box<Send + 'static>;
pub type LocalSender = mpsc::Sender<LocalMessage>;
pub type LocalReceiver = mpsc::Receiver<LocalMessage>;
