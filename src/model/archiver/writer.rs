use std::{
    fs::File,
    io::{BufWriter, Result, Write},
    mem,
};

use tokio::sync::{mpsc, oneshot};

use crate::model::Contact;

enum Command {
    Write(Contact),
    Finish(oneshot::Sender<Result<()>>),
}

pub struct Writer {
    commands: mpsc::UnboundedSender<Command>,
}

impl Writer {
    pub async fn new() -> Result<Writer> {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut inner = tokio::task::spawn_blocking(|| Inner::new(rx))
            .await
            .unwrap()?;

        tokio::task::spawn_blocking(move || inner.work());

        Ok(Self { commands: tx })
    }

    pub fn write(&self, contact: Contact) {
        self.commands.send(Command::Write(contact)).unwrap();
    }

    pub async fn flush(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.commands.send(Command::Finish(tx)).unwrap();
        rx.await.unwrap()
    }
}

struct Inner {
    commands: mpsc::UnboundedReceiver<Command>,
    res: Result<()>,
    file: BufWriter<File>,
}

impl Inner {
    fn new(commands: mpsc::UnboundedReceiver<Command>) -> Result<Self> {
        Ok(Self {
            commands,
            res: Ok(()),
            file: BufWriter::new(File::create("run/export.csv")?),
        })
    }

    fn work(&mut self) {
        loop {
            let Some(command) = self.commands.blocking_recv() else {
                break;
            };
            if let Some(res) = self.handle(command) {
                self.res = res
            }
        }
    }

    fn handle(&mut self, command: Command) -> Option<Result<()>> {
        match command {
            Command::Write(contact) if self.res.is_ok() => Some(self.write(contact)),
            Command::Write(_) => None,
            Command::Finish(result) => {
                match mem::replace(&mut self.res, Ok(())) {
                    Ok(_) => {
                        let res = self.finish();
                        result.send(res).unwrap()
                    }
                    Err(err) => result.send(Err(err)).unwrap(),
                }
                Some(Ok(()))
            }
        }
    }

    fn write(
        &mut self,
        Contact {
            id,
            first,
            last,
            phone,
            email,
        }: Contact,
    ) -> Result<()> {
        write!(self.file, "{id},{first},{last},{phone},{email}")
    }

    fn finish(&mut self) -> Result<()> {
        self.file.flush()
    }
}
