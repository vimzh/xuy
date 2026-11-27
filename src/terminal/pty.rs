//! Runs the user's shell on a PTY and reaps it on exit.

use std::{
    env,
    io::{self, Read, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::mpsc;

pub(super) enum Output {
    Bytes(Vec<u8>),
    Exit(String),
    Error(String),
}

pub(super) struct Session {
    master: Box<dyn MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    input: mpsc::UnboundedSender<Vec<u8>>,
    exited: Arc<AtomicBool>,
}

impl Session {
    pub(super) fn start(
        folder: &Path,
        cols: u16,
        rows: u16,
        output: mpsc::Sender<Output>,
    ) -> io::Result<Self> {
        let pty = native_pty_system()
            .openpty(size(cols, rows))
            .map_err(io::Error::other)?;
        let mut reader = pty.master.try_clone_reader().map_err(io::Error::other)?;
        let mut writer = pty.master.take_writer().map_err(io::Error::other)?;
        let mut shell = CommandBuilder::new(
            env::var_os("SHELL")
                .filter(|shell| !shell.is_empty())
                .unwrap_or_else(|| "/bin/zsh".into()),
        );
        shell.cwd(folder);
        shell.arg("-l");
        shell.env("TERM", "xterm-256color");
        shell.env("COLORTERM", "truecolor");
        let mut child = pty.slave.spawn_command(shell).map_err(io::Error::other)?;
        drop(pty.slave);
        let killer = child.clone_killer();
        let exited = Arc::new(AtomicBool::new(false));
        let done = exited.clone();
        let (input, mut receive) = mpsc::unbounded_channel::<Vec<u8>>();
        let read_output = output.clone();
        std::thread::spawn(move || {
            let mut buffer = [0; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if read_output
                            .blocking_send(Output::Bytes(buffer[..count].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    #[cfg(target_os = "macos")]
                    Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
                    Err(error) => {
                        let _ = read_output
                            .blocking_send(Output::Error(format!("Terminal read failed: {error}")));
                        break;
                    }
                }
            }
        });
        let write_output = output.clone();
        let writer_exited = exited.clone();
        std::thread::spawn(move || {
            while let Some(bytes) = receive.blocking_recv() {
                if let Err(error) = writer.write_all(&bytes).and_then(|_| writer.flush()) {
                    if !writer_exited.load(Ordering::Acquire) {
                        let _ = write_output.blocking_send(Output::Error(format!(
                            "Terminal write failed: {error}"
                        )));
                    }
                    break;
                }
            }
        });
        std::thread::spawn(move || {
            let result = child.wait();
            done.store(true, Ordering::Release);
            let message = match result {
                Ok(exit) => format!("Shell exited ({exit})"),
                Err(error) => format!("Could not wait for shell: {error}"),
            };
            let _ = output.blocking_send(Output::Exit(message));
        });
        Ok(Self {
            master: pty.master,
            killer,
            input,
            exited,
        })
    }

    pub(super) fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        self.master
            .resize(size(cols, rows))
            .map_err(|error| format!("Could not resize terminal: {error}"))
    }

    pub(super) fn input(&self, bytes: Vec<u8>) -> Result<(), String> {
        if self.exited.load(Ordering::Acquire) {
            return Ok(());
        }
        self.input
            .send(bytes)
            .map_err(|_| "The terminal input stream closed".into())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(fd) = self.master.as_raw_fd() {
            // Interactive shells put foreground commands in a separate process group.
            let foreground = unsafe { libc::tcgetpgrp(fd) };
            if foreground > 1 && foreground != unsafe { libc::getpgrp() } {
                unsafe { libc::kill(-foreground, libc::SIGHUP) };
            }
        }
        if !self.exited.load(Ordering::Acquire) {
            let _ = self.killer.kill();
        }
    }
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.clamp(1, 300),
        cols: cols.clamp(2, 500),
        pixel_width: 0,
        pixel_height: 0,
    }
}
