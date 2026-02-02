use awattar::awattar_mock::AwattarApiMock;
use config::config;
use fronius::FroniusMock;
use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};
use tungstenite::{WebSocket, connect, stream::MaybeTlsStream};

//-------------------------------------------------------------------------------------------------

pub struct IntegrationTest {
    pub config: config::Config,
    join_handles: Vec<JoinHandle<()>>,
    pub fronius_mock: Arc<Mutex<FroniusMock>>,
    pub awattar_mock: Arc<Mutex<AwattarApiMock>>,
}

//-------------------------------------------------------------------------------------------------

impl IntegrationTest {
    pub fn new(config: config::Config) -> Self {
        let _ = env_logger::try_init();
        Self {
            config,
            join_handles: vec![],
            fronius_mock: Arc::new(Mutex::new(FroniusMock::default())),
            awattar_mock: Arc::new(Mutex::new(AwattarApiMock::default())),
        }
    }

    pub fn setup(&mut self) -> WebSocket<MaybeTlsStream<TcpStream>> {
        let config_clone = self.config.clone();
        let fronius_mock_handle = Arc::clone(&self.fronius_mock);
        let awattar_mock_handle = Arc::clone(&self.awattar_mock);

        self.join_handles.push(spawn(move || {
            let hooks = Arc::new(Mutex::new(ocppcentral_system::hooks::OcppHooks::new(
                fronius_mock_handle,
                awattar_mock_handle,
                config_clone.clone(),
            )));

            ocpp::run::<ocppcentral_system::hooks::OcppHooks<FroniusMock, AwattarApiMock>>(
                &config_clone,
                Arc::clone(&hooks),
            )
            .expect("Could not run OCPPCentralSystem");
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
