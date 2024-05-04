use std::{ops::Deref, sync::Arc};

use futures::stream::StreamExt;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    sync::watch,
    task::AbortHandle,
};
use tracing::{info, info_span, instrument, trace, Instrument as _};

use crate::model::{Contact, Result};

use super::{Contacts, Error};

#[derive(Clone, Debug)]
pub struct Archiver {
    contacts: Contacts,
    state: watch::Sender<ArchiverState>,
    _recv: watch::Receiver<ArchiverState>,
}

#[derive(Debug)]
enum ArchiverState {
    Waiting,
    Running {
        progress: f32,
        abort_handle: AbortHandle,
    },
    Complete(Result<(), Arc<Error>>),
}

#[derive(Clone)]
pub enum ArchiverStatus {
    Waiting,
    Running(f32),
    Complete(Result<(), Arc<Error>>),
}

impl Archiver {
    pub fn new(contacts: Contacts) -> Self {
        let (state, _recv) = watch::channel(ArchiverState::Waiting);
        Self {
            contacts,
            state,
            _recv,
        }
    }

    pub async fn status(&self) -> ArchiverStatus {
        match self.state.borrow().deref() {
            ArchiverState::Waiting => ArchiverStatus::Waiting,
            ArchiverState::Running { progress, .. } => ArchiverStatus::Running(*progress),
            ArchiverState::Complete(result) => ArchiverStatus::Complete(result.clone()),
        }
    }

    async fn work(state: watch::Sender<ArchiverState>, contacts: Contacts, out_file: File) {
        info!("spawned worker");
        let state2 = state.clone();
        let mut out_file = BufWriter::new(out_file);
        let res: Result<()> = async move {
            let count = contacts.count().await?;
            let mut contacts = contacts.get_all().enumerate();
            while let Some((
                written,
                Ok(Contact {
                    id,
                    first,
                    last,
                    phone,
                    email,
                }),
            )) = contacts.next().await
            {
                // tokio::time::sleep(Duration::from_millis(1)).await;
                out_file
                    .write_all(format!("{id},{first},{last},{phone},{email}\n").as_bytes())
                    .await?;
                if written & 0x3FFF == 0 {
                    let progress = written as f32 / count as f32;
                    state2.send_modify(|s| {
                        if let ArchiverState::Running {
                            progress: ref mut p,
                            ..
                        } = s
                        {
                            *p = progress;
                        }
                    });
                    trace!(progress, "Updating progress");
                }
            }
            Ok(out_file.shutdown().await?)
        }
        .await;
        state
            .send(ArchiverState::Complete(res.map_err(Into::into)))
            .unwrap();

        info!("Finished work");
    }

    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        if let ArchiverState::Running { .. } = self.state.borrow().deref() {
            return Err(Error::ArchiverRunning);
        }
        let mut out_file = File::create("run/export.csv").await?;
        out_file
            .write_all(b"id,firstname,lastname,phone,email\n")
            .await?;

        let handle = tokio::spawn(
            Self::work(self.state.clone(), self.contacts.clone(), out_file)
                .instrument(info_span!("worker thread")),
        );
        info!("spawned worker thread");

        self.state
            .send(ArchiverState::Running {
                progress: 0.0,
                abort_handle: handle.abort_handle(),
            })
            .unwrap();

        Ok(())
    }

    pub async fn reset(&self) {
        if let ArchiverState::Running { abort_handle, .. } = self.state.borrow().deref() {
            abort_handle.abort();
        };
        self.state.send(ArchiverState::Waiting).unwrap();
    }
}
