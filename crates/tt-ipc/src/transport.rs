//! Length-prefixed JSON frames over a local socket, and a blocking client.
//!
//! A frame is a little-endian `u32` byte count followed by that many bytes of
//! JSON. Frames above [`MAX_FRAME`] are refused so a bad peer can't make
//! either side allocate without bound.

use crate::{ClientKind, ClientMsg, Event, PROTOCOL_VERSION, Request, Response, ServerMsg};
use interprocess::local_socket::{GenericNamespaced, Name, Stream, ToNsName, prelude::*};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::io::{self, BufReader, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const MAX_FRAME: usize = 64 * 1024 * 1024;

pub fn write_frame<W: Write, T: Serialize>(w: &mut W, msg: &T) -> io::Result<()> {
    let body = serde_json::to_vec(msg).map_err(io::Error::other)?;
    if body.len() > MAX_FRAME {
        return Err(io::Error::other("frame too large"));
    }
    let len = (body.len() as u32).to_le_bytes();
    w.write_all(&len)?;
    w.write_all(&body)?;
    w.flush()
}

pub fn read_frame<R: Read, T: DeserializeOwned>(r: &mut R) -> io::Result<T> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let n = u32::from_le_bytes(len) as usize;
    if n > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut body = vec![0u8; n];
    r.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// The socket name for an engine instance. `instance` separates a dev or
/// benchmark engine (its own data folder) from the user's real one.
pub fn socket_name(instance: &str) -> io::Result<Name<'static>> {
    format!("ttorrent-{instance}.sock").to_ns_name::<GenericNamespaced>()
}

/// A short stable id for a data folder, used as the instance name.
pub fn instance_id(data_dir: &std::path::Path) -> String {
    // FNV-1a: stable across runs and Rust versions (unlike DefaultHasher).
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data_dir.to_string_lossy().to_lowercase().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", h as u32)
}

type Pending = Arc<Mutex<HashMap<u64, Sender<Result<Response, String>>>>>;

/// A blocking connection to the engine. Events arrive on the receiver returned
/// by [`Client::connect`]; calls can be made from any thread.
pub struct Client {
    writer: Mutex<Shared>,
    pending: Pending,
    next: AtomicU64,
    pub engine_pid: u32,
}

impl Client {
    pub fn connect(instance: &str, client: ClientKind) -> io::Result<(Client, Receiver<Event>)> {
        let stream = Arc::new(Stream::connect(socket_name(instance)?)?);
        // One pipe shared by a reader thread and the writer here (`&Stream` is Read + Write).
        let mut send_stream = Shared(stream.clone());
        write_frame(
            &mut send_stream,
            &ClientMsg::Hello {
                version: PROTOCOL_VERSION,
                client,
            },
        )?;
        let mut reader = BufReader::new(Shared(stream));
        let engine_pid = match read_frame::<_, ServerMsg>(&mut reader)? {
            ServerMsg::Hello {
                version,
                engine_pid,
            } if version == PROTOCOL_VERSION => engine_pid,
            ServerMsg::Hello { version, .. } => {
                return Err(io::Error::other(format!(
                    "engine speaks protocol {version}, this build speaks {PROTOCOL_VERSION}"
                )));
            }
            _ => return Err(io::Error::other("engine did not say hello")),
        };
        let pending: Pending = Arc::default();
        let (ev_tx, ev_rx) = mpsc::channel();
        let p = pending.clone();
        std::thread::Builder::new()
            .name("tt-ipc-reader".into())
            .spawn(move || {
                loop {
                    match read_frame::<_, ServerMsg>(&mut reader) {
                        Ok(ServerMsg::Reply { id, result }) => {
                            if let Ok(mut map) = p.lock()
                                && let Some(tx) = map.remove(&id)
                            {
                                let _ = tx.send(result);
                            }
                        }
                        Ok(ServerMsg::Event { event }) => {
                            if ev_tx.send(event).is_err() {
                                break;
                            }
                        }
                        Ok(ServerMsg::Hello { .. }) => {}
                        Err(_) => {
                            // Engine gone: fail every waiting call, end the event stream.
                            if let Ok(mut map) = p.lock() {
                                for (_, tx) in map.drain() {
                                    let _ = tx.send(Err("engine closed the connection".into()));
                                }
                            }
                            let _ = ev_tx.send(Event::Exiting);
                            break;
                        }
                    }
                }
            })?;
        Ok((
            Client {
                writer: Mutex::new(send_stream),
                pending,
                next: AtomicU64::new(1),
                engine_pid,
            },
            ev_rx,
        ))
    }

    /// Send a request and wait for its reply (30 s at most).
    pub fn call(&self, request: Request) -> Result<Response, String> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        if let Ok(mut map) = self.pending.lock() {
            map.insert(id, tx);
        }
        {
            let mut w = self.writer.lock().map_err(|e| e.to_string())?;
            write_frame(&mut *w, &ClientMsg::Request { id, request }).map_err(|e| e.to_string())?;
        }
        rx.recv_timeout(Duration::from_secs(30))
            .map_err(|_| "the engine did not answer".to_string())?
    }
}

/// A stream shared between threads: reads and writes go through `&Stream`.
pub struct Shared(pub Arc<Stream>);

impl Read for Shared {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&*self.0).read(buf)
    }
}

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self.0).write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        (&*self.0).flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TorrentRow, TorrentState};

    #[test]
    fn frames_round_trip() {
        let msg = ServerMsg::Event {
            event: Event::Rows {
                upsert: vec![TorrentRow {
                    id: 7,
                    name: "debian-13.2.0-amd64-netinst.iso".into(),
                    state: TorrentState::Seeding,
                    progress: 1.0,
                    ..Default::default()
                }],
                removed: vec![3],
            },
        };
        let mut buf = Vec::new();
        write_frame(&mut buf, &msg).unwrap();
        let back: ServerMsg = read_frame(&mut buf.as_slice()).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn oversized_frame_refused() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(u32::MAX).to_le_bytes());
        let r: io::Result<ServerMsg> = read_frame(&mut buf.as_slice());
        assert!(r.is_err());
    }

    #[test]
    fn instance_ids_are_stable() {
        let a = instance_id(std::path::Path::new("C:/Users/x/AppData/Roaming/tTorrent"));
        let b = instance_id(std::path::Path::new("c:/users/x/appdata/roaming/ttorrent"));
        assert_eq!(a, b);
        assert_eq!(a.len(), 8);
    }

    proptest::proptest! {
        #[test]
        fn requests_round_trip(ids in proptest::collection::vec(0u32..10_000, 0..50), del in proptest::bool::ANY) {
            let msg = ClientMsg::Request { id: 9, request: Request::Remove { ids, delete_files: del } };
            let mut buf = Vec::new();
            write_frame(&mut buf, &msg).unwrap();
            let back: ClientMsg = read_frame(&mut buf.as_slice()).unwrap();
            proptest::prop_assert_eq!(msg, back);
        }
    }
}
