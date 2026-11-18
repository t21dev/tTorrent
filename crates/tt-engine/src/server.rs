//! The local socket the window, a second launch and the bench talk to.
//! One reader thread and one writer thread per connection; requests run on
//! the engine's runtime, replies and events share the writer.

use crate::core::Engine;
use interprocess::local_socket::{ListenerOptions, prelude::*};
use std::io::BufReader;
use std::sync::Arc;
use std::sync::mpsc;
use tt_ipc::transport::{Shared, read_frame, socket_name, write_frame};
use tt_ipc::{ClientKind, ClientMsg, PROTOCOL_VERSION, Request, ServerMsg};

/// Claim the instance's socket. Fails if another engine already owns it.
pub fn listen(instance: &str) -> std::io::Result<interprocess::local_socket::Listener> {
    ListenerOptions::new()
        .name(socket_name(instance)?)
        .create_sync()
}

pub fn serve(engine: Arc<Engine>, listener: interprocess::local_socket::Listener) {
    let _ = std::thread::Builder::new()
        .name("tt-ipc-accept".into())
        .spawn(move || {
            for conn in listener.incoming() {
                match conn {
                    Ok(c) => {
                        let e = engine.clone();
                        let _ = std::thread::Builder::new()
                            .name("tt-ipc-conn".into())
                            .spawn(move || connection(e, Arc::new(c)));
                    }
                    Err(e) => tracing::warn!("ipc accept: {e}"),
                }
            }
        });
}

fn connection(engine: Arc<Engine>, conn: Arc<interprocess::local_socket::Stream>) {
    let mut reader = BufReader::new(Shared(conn.clone()));
    let client = match read_frame::<_, ClientMsg>(&mut reader) {
        Ok(ClientMsg::Hello { client, .. }) => client,
        _ => return,
    };
    let (tx, rx) = mpsc::channel::<ServerMsg>();
    let mut w = Shared(conn);
    let writer = std::thread::Builder::new()
        .name("tt-ipc-write".into())
        .spawn(move || {
            while let Ok(msg) = rx.recv() {
                if write_frame(&mut w, &msg).is_err() {
                    break;
                }
            }
        });
    if writer.is_err() {
        return;
    }
    let _ = tx.send(ServerMsg::Hello {
        version: PROTOCOL_VERSION,
        engine_pid: std::process::id(),
    });
    let is_window = client == ClientKind::Window;
    while let Ok(msg) = read_frame::<_, ClientMsg>(&mut reader) {
        let ClientMsg::Request { id, request } = msg else {
            continue;
        };
        if request == Request::Subscribe {
            let events = engine.subscribe();
            let tx2 = tx.clone();
            let _ = std::thread::Builder::new()
                .name("tt-ipc-events".into())
                .spawn(move || {
                    while let Ok(event) = events.recv() {
                        if tx2.send(ServerMsg::Event { event }).is_err() {
                            break;
                        }
                    }
                });
        }
        let e = engine.clone();
        let tx2 = tx.clone();
        engine.rt.spawn(async move {
            let result = e.handle(request).await.map_err(|err| format!("{err:#}"));
            let _ = tx2.send(ServerMsg::Reply { id, result });
        });
    }
    if is_window {
        tracing::debug!("window disconnected");
    }
}
