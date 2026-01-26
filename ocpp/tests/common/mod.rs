use config::config;
use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};
use tungstenite::{WebSocket, connect, stream::MaybeTlsStream};

use std::path::Path;

use crate::Hook;

//-------------------------------------------------------------------------------------------------

pub struct IntegrationTest {
    pub config: config::Config,
    join_handles: Vec<JoinHandle<()>>,
    hook: Arc<Mutex<Hook>>,
}

//-------------------------------------------------------------------------------------------------

impl IntegrationTest {
    pub fn new(config: config::Config, hook: Arc<Mutex<Hook>>) -> Self {
        Self {
            config,
            join_handles: vec![],
            hook,
        }
    }

    pub fn setup(&mut self) -> WebSocket<MaybeTlsStream<TcpStream>> {
        let thread_safe_hook = self.hook.clone();
        let config_clone = self.config.clone();

        self.join_handles.push(spawn(move || {
            ocpp::run::<Hook>(&config_clone, thread_safe_hook)
                .expect("OCPP central system could not be started!")
        }));

        let websocket_address = format!(
            "ws://{}:{}",
            self.config.websocket.ip, self.config.websocket.port
        );

        // Websocket startup might take some time
        for i in 0..20 {
            match connect(websocket_address.to_owned()) {
                Ok((socket, _)) => {
                    return socket;
                }
                _ => {}
            }

            std::thread::sleep(std::time::Duration::from_secs(i));
        }

        panic!("Could not connect!");
    }

    pub fn teardown(
        self,
        log_directory: &str,
        websocket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    ) {
        websocket
            .write(tungstenite::Message::Close(None))
            .expect("Could not close connection!");

        for handle in self.join_handles {
            handle.join().expect("Could not join thread!");
        }

        std::fs::remove_dir_all(log_directory).expect("Cleanup failed");
    }
}
