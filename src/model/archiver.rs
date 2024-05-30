use std::{future, pin::Pin, sync::Arc, task::Poll};

use futures::{
    stream::{BoxStream, StreamExt},
    Stream,
};
use tokio::sync::{mpsc, oneshot};
use tracing::{info, instrument};

use crate::model::{Contact, Contacts, Error, Result};

use self::writer::Writer;

mod writer;

#[derive(Clone, Debug)]
pub struct Archiver {
    commands: mpsc::Sender<Command>,
}

#[derive(Clone)]
pub enum ArchiverStatus {
    Waiting,
    Running(f32),
    Complete(Result<(), Arc<Error>>),
}

enum Command {
    Start,
    Reset,
    GetStatus(oneshot::Sender<ArchiverStatus>),
}

impl Archiver {
    pub async fn new(contacts: Contacts) -> Result<Self> {
        let (commands, recv) = mpsc::channel(1);
        let inner = Inner::new(recv, contacts).await?;
        tokio::spawn(inner.work());
        Ok(Self { commands })
    }

    pub async fn status(&self) -> Result<ArchiverStatus> {
        let (tx, rx) = oneshot::channel();
        self.commands.send(Command::GetStatus(tx)).await?;
        rx.await.map_err(Into::into)
    }

    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        Ok(self.commands.send(Command::Start).await?)
    }

    pub async fn reset(&self) -> Result<()> {
        Ok(self.commands.send(Command::Reset).await?)
    }
}

#[ouroboros::self_referencing]
struct StreamWrapper {
    contacts: Contacts,
    #[borrows(contacts)]
    #[covariant]
    #[pin]
    rows: BoxStream<'this, Result<Contact, Error>>,
}

impl Unpin for StreamWrapper {}

impl Stream for StreamWrapper {
    type Item = Result<Contact, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut()
            .with_rows_mut(|rows| Pin::new(rows).poll_next(cx))
    }
}

struct Inner {
    commands: mpsc::Receiver<Command>,
    contacts: Contacts,
    state: State,
}

enum State {
    Waiting,
    Running(Running),
    Complete(Option<Arc<Error>>),
}

struct Running {
    stream: StreamWrapper,
    count: u64,
    total: u64,
    writer: Writer,
}

impl Inner {
    async fn new(commands: mpsc::Receiver<Command>, contacts: Contacts) -> Result<Self> {
        Ok(Inner {
            commands,
            contacts,
            state: State::Waiting,
        })
    }

    async fn work(mut self) {
        info!("spawned worker");
        loop {
            if let State::Running(mut running) = self.state {
                tokio::select! {
                    biased;
                    command = self.commands.recv() => {
                        match command {
                            Some(command) => self.state = State::Running(running).handle_command(&self.contacts, command).await,
                            None => break,
                        }
                    }

                    row = running.stream.next() => {
                        self.state = running.handle_row(row).await;
                    }
                }
            } else {
                tokio::select! {
                    command = self.commands.recv() => {
                        match command {
                            Some(command) => self.state = self.state.handle_command(&self.contacts, command).await,
                            None => break,
                        }
                    }
                }
            }
        }
    }
}

impl State {
    async fn handle_command(self, contacts: &Contacts, command: Command) -> State {
        let res = match (self, command) {
            (State::Waiting, Command::Start) => Self::start(contacts).await,
            (s @ State::Waiting, Command::Reset) => Ok(s),
            (s @ State::Waiting, Command::GetStatus(ret)) => ret
                .send(ArchiverStatus::Waiting)
                .map_err(|_| Error::CommandSendFailed)
                .map(|_| s),
            (s @ State::Running(_), Command::Start) => Ok(s),
            (State::Running(_), Command::Reset) => Ok(State::Waiting),
            (s @ State::Running(Running { count, total, .. }), Command::GetStatus(ret)) => ret
                .send(ArchiverStatus::Running(count as f32 / total as f32))
                .map_err(|_| Error::CommandSendFailed)
                .map(|_| s),
            (State::Complete(_), Command::Start) => Self::start(contacts).await,
            (State::Complete(_), Command::Reset) => Ok(Self::Waiting),
            (State::Complete(err), Command::GetStatus(ret)) => ret
                .send(ArchiverStatus::Complete(
                    err.clone().map(Err).unwrap_or(Ok(())),
                ))
                .map_err(|_| Error::CommandSendFailed)
                .map(|_| Self::Complete(err)),
        };
        match res {
            Ok(state) => state,
            Err(err) => State::Complete(Some(Arc::new(err))),
        }
    }

    async fn start(contacts: &Contacts) -> Result<Self> {
        Ok(Self::Running(Running {
            total: contacts.count().await?,
            stream: StreamWrapper::new(contacts.clone(), |c| Box::pin(c.get_all())),
            count: 0,
            writer: Writer::new().await?,
        }))
    }
}

impl Running {
    async fn handle_row(mut self, row: Option<Result<Contact>>) -> State {
        let Some(row) = row else {
            let res = self.writer.flush().await;
            return State::Complete(res.err().map(|e| Arc::new(e.into())));
        };
        match row {
            Ok(contact) => {
                self.writer.write(contact);
                self.count += 1;

                // Yield every once in a while.
                // Otherwise we somehow lock the entire server.
                if self.count & 0xFF == 0 {
                    yield_once().await;
                }
                State::Running(self)
            }
            Err(err) => State::Complete(Some(Arc::new(err))),
        }
    }
}

async fn yield_once() {
    let mut pending = true;
    future::poll_fn(|ctx| {
        if pending {
            pending = false;
            ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
}
