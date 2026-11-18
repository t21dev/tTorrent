//! A real local socket: the server keeps a read blocked while another thread
//! writes events on the same pipe. A synchronous Windows pipe handle would
//! deadlock here; this proves ours doesn't.

use interprocess::local_socket::{ListenerOptions, prelude::*};
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;
use tt_ipc::transport::{Client, Shared, read_frame, socket_name, write_frame};
use tt_ipc::{ClientKind, ClientMsg, Event, PROTOCOL_VERSION, Request, Response, ServerMsg};

#[test]
fn events_flow_while_server_reads() {
    let instance = format!("test-{}", std::process::id());
    let listener = ListenerOptions::new()
        .name(socket_name(&instance).unwrap())
        .create_sync()
        .unwrap();
    let server = std::thread::spawn(move || {
        let conn = Arc::new(listener.incoming().next().unwrap().unwrap());
        let mut reader = BufReader::new(Shared(conn.clone()));
        let hello: ClientMsg = read_frame(&mut reader).unwrap();
        assert!(matches!(hello, ClientMsg::Hello { .. }));
        let mut w = Shared(conn.clone());
        write_frame(
            &mut w,
            &ServerMsg::Hello {
                version: PROTOCOL_VERSION,
                engine_pid: 1,
            },
        )
        .unwrap();
        // Events from another thread while this one blocks in read.
        let mut w2 = Shared(conn.clone());
        std::thread::spawn(move || {
            for i in 0..50 {
                write_frame(
                    &mut w2,
                    &ServerMsg::Event {
                        event: Event::Notice {
                            text: format!("n{i}"),
                        },
                    },
                )
                .unwrap();
            }
        });
        let req: ClientMsg = read_frame(&mut reader).unwrap();
        if let ClientMsg::Request { id, .. } = req {
            write_frame(
                &mut w,
                &ServerMsg::Reply {
                    id,
                    result: Ok(Response::Ok),
                },
            )
            .unwrap();
        }
    });
    std::thread::sleep(Duration::from_millis(50));
    let (client, events) = Client::connect(&instance, ClientKind::Bench).unwrap();
    let mut got = 0;
    while got < 50 {
        match events.recv_timeout(Duration::from_secs(5)).unwrap() {
            Event::Notice { .. } => got += 1,
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(client.call(Request::Snapshot), Ok(Response::Ok));
    server.join().unwrap();
}
