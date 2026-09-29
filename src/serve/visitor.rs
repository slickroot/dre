use super::session::{Session, Spawner, Window};
use russh::keys::ssh_key::{HashAlg, PublicKey};
use russh::server::{Auth, Handle, Handler, Msg, Server};
use russh::{Channel, ChannelId};
use std::future::Future;
use std::sync::{Arc, Weak};

pub(crate) trait Outlet: Clone + Send + 'static {
    fn send(&self, bytes: Vec<u8>) -> impl Future<Output = ()> + Send;
    fn close(&self) -> impl Future<Output = ()> + Send;
}

#[derive(Clone)]
pub(crate) struct ChannelOutlet {
    handle: Handle,
    channel: ChannelId,
}

impl Outlet for ChannelOutlet {
    async fn send(&self, bytes: Vec<u8>) {
        let _ = self.handle.data(self.channel, bytes).await;
    }

    async fn close(&self) {
        let _ = self.handle.close(self.channel).await;
    }
}

pub(crate) struct SshServer {
    spawner: Arc<dyn Spawner + Send + Sync>,
}

impl SshServer {
    pub(crate) fn new(spawner: Arc<dyn Spawner + Send + Sync>) -> Self {
        Self { spawner }
    }
}

impl Server for SshServer {
    type Handler = VisitorHandler;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> VisitorHandler {
        VisitorHandler::new(self.spawner.clone())
    }
}

pub(crate) struct VisitorHandler {
    spawner: Arc<dyn Spawner + Send + Sync>,
    #[cfg_attr(not(test), allow(dead_code))]
    fingerprint: Option<String>,
    window: Window,
    session: Option<Arc<dyn Session>>,
}

impl VisitorHandler {
    pub(crate) fn new(spawner: Arc<dyn Spawner + Send + Sync>) -> Self {
        Self {
            spawner,
            fingerprint: None,
            window: Window::default(),
            session: None,
        }
    }

    fn record_key(&mut self, key: &PublicKey) {
        self.fingerprint = Some(key.fingerprint(HashAlg::Sha256).to_string());
    }

    fn start_shell(&mut self, outlet: impl Outlet) -> std::io::Result<()> {
        let session: Arc<dyn Session> = Arc::from(self.spawner.spawn(self.window)?);
        tokio::spawn(relay(Arc::downgrade(&session), outlet));
        self.session = Some(session);
        Ok(())
    }

    fn write(&self, bytes: &[u8]) {
        if let Some(session) = &self.session {
            session.write(bytes);
        }
    }

    fn resize(&mut self, window: Window) {
        self.window = window;
        if let Some(session) = &self.session {
            session.resize(window);
        }
    }
}

async fn relay(session: Weak<dyn Session>, outlet: impl Outlet) {
    while let Some(session) = session.upgrade() {
        let Ok(Some(bytes)) = tokio::task::spawn_blocking(move || session.read()).await else {
            break;
        };
        outlet.send(bytes).await;
    }
    outlet.close().await;
}

fn window(cols: u32, rows: u32, pixel_width: u32, pixel_height: u32) -> Window {
    Window {
        cols,
        rows,
        pixel_width,
        pixel_height,
    }
}

impl Handler for VisitorHandler {
    type Error = russh::Error;

    async fn auth_publickey(&mut self, _user: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        self.record_key(key);
        Ok(Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        cols: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        self.window = window(cols, rows, pixel_width, pixel_height);
        session.channel_success(channel)
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        let outlet = ChannelOutlet {
            handle: session.handle(),
            channel,
        };
        self.start_shell(outlet)?;
        session.channel_success(channel)
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        self.write(data);
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _channel: ChannelId,
        cols: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
        _session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        self.resize(window(cols, rows, pixel_width, pixel_height));
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::session::{MockSession, MockSpawner};
    use russh::keys::{Algorithm, PrivateKey};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    const SIZE: Window = Window {
        cols: 80,
        rows: 24,
        pixel_width: 800,
        pixel_height: 480,
    };

    #[derive(Clone)]
    struct FakeOutlet {
        sent: mpsc::UnboundedSender<Option<Vec<u8>>>,
    }

    impl Outlet for FakeOutlet {
        async fn send(&self, bytes: Vec<u8>) {
            self.sent.send(Some(bytes)).unwrap();
        }

        async fn close(&self) {
            self.sent.send(None).unwrap();
        }
    }

    fn fake_outlet() -> (FakeOutlet, mpsc::UnboundedReceiver<Option<Vec<u8>>>) {
        let (sent, received) = mpsc::unbounded_channel();
        (FakeOutlet { sent }, received)
    }

    fn public_key() -> PublicKey {
        PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)
            .unwrap()
            .public_key()
            .clone()
    }

    fn silent_session() -> MockSession {
        let mut session = MockSession::new();
        session.expect_read().returning(|| None);
        session
    }

    fn spawner_of(session: MockSession) -> Arc<dyn Spawner + Send + Sync> {
        let session = Mutex::new(Some(session));
        let mut spawner = MockSpawner::new();
        spawner.expect_spawn().returning(move |_| {
            Ok(Box::new(session.lock().unwrap().take().unwrap()) as Box<dyn Session>)
        });
        Arc::new(spawner)
    }

    fn started_handler(session: MockSession) -> VisitorHandler {
        let mut handler = VisitorHandler::new(spawner_of(session));
        handler.start_shell(fake_outlet().0).unwrap();
        handler
    }

    #[tokio::test]
    async fn any_public_key_is_accepted() {
        let mut handler = VisitorHandler::new(Arc::new(MockSpawner::new()));
        let key = public_key();
        let auth = handler.auth_publickey("anyone", &key).await.unwrap();
        assert!(matches!(auth, Auth::Accept));
        assert_eq!(
            handler.fingerprint,
            Some(key.fingerprint(HashAlg::Sha256).to_string())
        );
    }

    #[tokio::test]
    async fn shell_request_spawns_with_the_size_from_pty_request() {
        let mut spawner = MockSpawner::new();
        spawner
            .expect_spawn()
            .withf(|window| *window == SIZE)
            .times(1)
            .returning(|_| Ok(Box::new(silent_session())));
        let mut handler = VisitorHandler::new(Arc::new(spawner));
        handler.window = SIZE;
        handler.start_shell(fake_outlet().0).unwrap();
    }

    #[tokio::test]
    async fn channel_data_reaches_the_session() {
        let mut session = silent_session();
        session
            .expect_write()
            .withf(|bytes| bytes == b"hello")
            .times(1)
            .return_const(());
        let handler = started_handler(session);
        handler.write(b"hello");
    }

    #[tokio::test]
    async fn session_output_reaches_the_channel() {
        let mut session = MockSession::new();
        let mut outputs = vec![None, Some(b"drawn".to_vec())];
        session
            .expect_read()
            .returning(move || outputs.pop().unwrap());
        let mut handler = VisitorHandler::new(spawner_of(session));
        let (outlet, mut received) = fake_outlet();
        handler.start_shell(outlet).unwrap();
        assert_eq!(received.recv().await, Some(Some(b"drawn".to_vec())));
    }

    #[tokio::test]
    async fn window_change_reaches_the_session() {
        let mut session = silent_session();
        session
            .expect_resize()
            .withf(|window| *window == SIZE)
            .times(1)
            .return_const(());
        let mut handler = started_handler(session);
        handler.resize(SIZE);
    }

    #[tokio::test]
    async fn end_of_session_output_closes_the_channel() {
        let mut handler = VisitorHandler::new(spawner_of(silent_session()));
        let (outlet, mut received) = fake_outlet();
        handler.start_shell(outlet).unwrap();
        assert_eq!(received.recv().await, Some(None));
    }

    #[tokio::test]
    async fn two_connections_get_two_separate_sessions() {
        let mut spawner = MockSpawner::new();
        spawner.expect_spawn().times(2).returning(|_| {
            let mut session = silent_session();
            session.expect_write().times(1).return_const(());
            Ok(Box::new(session))
        });
        let mut server = SshServer::new(Arc::new(spawner));
        let mut first = server.new_client(None);
        let mut second = server.new_client(None);
        first.start_shell(fake_outlet().0).unwrap();
        second.start_shell(fake_outlet().0).unwrap();
        first.write(b"a");
        second.write(b"b");
    }

    struct DropProbe(Arc<AtomicBool>);

    impl Session for DropProbe {
        fn write(&self, _bytes: &[u8]) {}
        fn read(&self) -> Option<Vec<u8>> {
            None
        }
        fn resize(&self, _window: Window) {}
        fn wait(&self) {}
    }

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn dropping_the_handler_drops_the_session() {
        let dropped = Arc::new(AtomicBool::new(false));
        let probe = Mutex::new(Some(DropProbe(dropped.clone())));
        let mut spawner = MockSpawner::new();
        spawner
            .expect_spawn()
            .returning(move |_| Ok(Box::new(probe.lock().unwrap().take().unwrap())));
        let mut handler = VisitorHandler::new(Arc::new(spawner));
        let (outlet, mut received) = fake_outlet();
        handler.start_shell(outlet).unwrap();
        received.recv().await;
        drop(handler);
        assert!(dropped.load(Ordering::SeqCst));
    }
}
