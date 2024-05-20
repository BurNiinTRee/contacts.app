use std::{io::Write as _, sync::Arc};

use futures::stream::{BoxStream, StreamExt};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    sync::{mpsc, oneshot},
};
use tracing::{info, instrument, trace};

use crate::model::{Contact, Contacts, Error, Result};

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
    pub fn new(contacts: Contacts) -> Self {
        let (commands, recv) = mpsc::channel(1);
        tokio::spawn(Self::work(recv, contacts));
        Self { commands }
    }

    pub async fn status(&self) -> Result<ArchiverStatus> {
        let (tx, rx) = oneshot::channel();
        self.commands.send(Command::GetStatus(tx)).await?;
        rx.await.map_err(Into::into)
    }

    async fn work(mut commands: mpsc::Receiver<Command>, contacts: Contacts) -> ! {
        info!("spawned worker");
        let mut rows: BoxStream<Result<Contact>> = Box::pin(futures::stream::pending());
        let mut running = false;
        let mut count = 0;
        let mut total = 0;
        let mut result: Option<Result<_, Arc<Error>>> = None;
        let mut file = None;
        let mut line = Vec::new();
        loop {
            tokio::select! {
                command = commands.recv() => {
                    match command {
                        Some(Command::Start)=> {
                            result = None;
                            running = true;
                            total = match contacts.count().await {
                                Ok(c) => c,
                                Err(err) => {
                                    result = Some(Err(err.into()));
                                    continue;
                                },
                            };
                            count = 0;
                            file = Some(match File::create("run/export.csv").await {
                                Ok(f) => BufWriter::new(f),
                                Err(err) => {
                                    result = Some(Err(Arc::new(Error::from(err))));
                                    continue;
                                },
                            });
                            let new_rows: BoxStream<Result<Contact>> = Box::pin(contacts.get_all());
                            rows = new_rows;
                            info!("Archiving started");
                        },
                        Some(Command::GetStatus(ret)) => {
                            let _ = match (&mut result, running) {
                                (Some(res), _) => ret.send(ArchiverStatus::Complete(res.clone().map_err(Into::into))),
                                (_, false) => ret.send(ArchiverStatus::Waiting),
                                _ => ret.send(ArchiverStatus::Running(count as f32 / total as f32))
                            };
                            trace!("Archiver returned status");
                        }
                        Some(Command::Reset) => {
                            result = None;
                            running = false;
                            info!("Archiver got reset");
                        }
                        None => {},
                    }
                }

                row = rows.next(), if running => {
                    match row {
                        Some(res) => match res {
                            Ok(Contact { id, first, last, phone, email }) => {
                                line.clear();
                                writeln!(&mut line, "{id},{first},{last},{phone},{email}").unwrap();

                                if let Err(err) = file.as_mut().expect("File not opened")
                                    .write_all(&line)
                                    .await {
                                    result = Some(Err(Arc::new(err.into())));
                                    continue;
                                }
                                count += 1;
                            },
                            Err(err) => {
                                result = Some(Err(err.into()));
                                continue;
                            },
                        },
                        None => {
                            if let Some(mut f) = file.take() { f.flush().await.unwrap() };
                            result = Some(Ok(()));
                        }
                    }
                }
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        Ok(self.commands.send(Command::Start).await?)
    }

    pub async fn reset(&self) -> Result<()> {
        Ok(self.commands.send(Command::Reset).await?)
    }
}
