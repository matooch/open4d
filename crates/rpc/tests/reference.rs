//! Every method in the generated RPC reference is answered by a real server
//! (docs/specs/RPC.md §11). Port 0.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use forge_app::App;
use forge_rpc::docs::METHODS;
use forge_rpc::{Client, ClientError, Server, ServerConfig, code};
use serde_json::json;

#[test]
fn every_documented_method_exists_and_an_undocumented_one_does_not() {
    let (server, inbox) = Server::start(ServerConfig {
        port: 0,
        token: Some("tok".into()),
        insecure_local: false,
    })
    .unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let pump = std::thread::spawn(move || {
        let mut host = App::new_default();
        inbox.run_until(&mut host, &flag);
    });
    let mut client = Client::connect(server.port(), Some("tok")).unwrap();
    for method in METHODS {
        // Empty params are valid or a parameter error, never "unknown method".
        match client.call(method.name, json!({})) {
            Ok(_) => {}
            Err(ClientError::Rpc { code: c, .. }) => {
                assert_ne!(c, code::METHOD_NOT_FOUND, "{} is documented", method.name);
            }
            Err(e) => panic!("{}: {e}", method.name),
        }
    }
    match client.call("no.such_method", json!({})) {
        Err(ClientError::Rpc { code: c, .. }) => assert_eq!(c, code::METHOD_NOT_FOUND),
        other => panic!("{other:?}"),
    }
    stop.store(true, Ordering::SeqCst);
    drop(client);
    pump.join().unwrap();
}
