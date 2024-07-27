use std::sync::Arc;
use bytes::Bytes;
use rsa::RsaPrivateKey;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use crate::server::connection::ClientConnection;
use crate::server::handler::{DefaultPacketHandler, PacketHandler};
use crate::server::text::TextComponent;
use crate::Tag;

pub struct Server {
    pub(crate) max_players: i32,
    pub(crate) motd: TextComponent,
    pub(crate) key: RsaPrivateKey,
    handler_factory: Box<dyn Fn() -> Box<dyn PacketHandler + Send>>,
}

unsafe impl Send for Server {

}

impl Server {
    pub fn new() -> Server {
        Server {
            max_players: 0,
            motd: TextComponent::plain("A Minecraft Server"),
            key: RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("failed to generate a key"),
            handler_factory: Box::new(|| Box::new(DefaultPacketHandler::new()))
        }
    }

    pub fn set_motd(&mut self, motd: TextComponent) {
        self.motd = motd;
    }

    pub fn set_handler_factory(&mut self, factory: impl Fn() -> Box<dyn PacketHandler + Send> + 'static) {
        self.handler_factory = Box::new(factory);
    }

    pub async fn start(mut self, addr: &str) -> anyhow::Result<()> {
        let self_arc = Arc::new(Mutex::new(self));
        let listener = TcpListener::bind(addr).await?;

        loop {
            // Asynchronously wait for an inbound socket.
            let (mut socket, addr) = listener.accept().await?;
            println!("client connected: {}", addr);

            let handler = {
                (self_arc.clone().lock().await.handler_factory)()
            };
            let moveself = self_arc.clone();
            tokio::spawn(async move {
                ClientConnection::new(handler, moveself).handle(socket).await;
            });
        }
    }
}
