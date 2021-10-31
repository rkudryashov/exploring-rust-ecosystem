use std::sync::Mutex;
use std::time::Duration;

use actix_web::web::{Bytes, Data};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::{task, time};

use crate::errors::CustomError;

#[derive(Clone)]
pub struct Broadcaster {
    clients: Vec<Sender<Result<Bytes, CustomError>>>,
}

impl Broadcaster {
    fn new() -> Self {
        Broadcaster {
            clients: Vec::new(),
        }
    }

    pub fn create() -> Data<Mutex<Self>> {
        let me = Data::new(Mutex::new(Broadcaster::new()));

        // ping clients every 10 seconds to see if they are alive
        Broadcaster::spawn_ping(me.clone());

        me
    }

    pub async fn new_client(&mut self) -> Receiver<Result<Bytes, CustomError>> {
        let (tx, rx) = mpsc::channel::<Result<Bytes, CustomError>>(100);

        tx.clone()
            .send(Ok(Bytes::from("data: Connected\n\n")))
            .await
            .expect("Can't create a client");

        self.clients.push(tx);
        crate::metrics::HTTP_CONNECTED_SSE_CLIENTS.inc();
        rx
    }

    pub fn send(&self, msg: Bytes) {
        for client in self.clients.iter() {
            client
                .try_send(Ok(msg.clone()))
                .expect("Can't send a message");
        }
    }

    fn spawn_ping(me: Data<Mutex<Self>>) {
        task::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;
                let mut broadcaster_mutex = me.lock().expect("Failed to lock broadcaster");
                broadcaster_mutex.remove_stale_clients();
                crate::metrics::HTTP_CONNECTED_SSE_CLIENTS
                    .set(broadcaster_mutex.clients.len() as i64)
            }
        });
    }

    fn remove_stale_clients(&mut self) {
        let mut ok_clients = Vec::new();
        for client in self.clients.iter() {
            let result = client.clone().try_send(Ok(Bytes::from("data: Ping\n\n")));

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }
}
