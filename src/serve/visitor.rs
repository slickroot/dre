use super::diagram_dir::DiagramDir;
use super::session::{Session, Spawner, Window};
use super::sessions::{Closer, Sessions, Token};
use russh::keys::ssh_key::{HashAlg, PublicKey};
use russh::server::{Auth, Handle, Handler, Msg, Response, Server};
use russh::{Channel, ChannelId};
use std::future::Future;
use std::sync::{Arc, Weak};

const PUBLIC_HOST: &str = "dre.elaich.com";

fn no_key_message() -> String {
    format!(
        "dre needs an SSH key to remember your diagram. Run `ssh-keygen`, then try `ssh {PUBLIC_HOST}` again.\r\n"
    )
}

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

struct OutletCloser<O>(O);

impl<O: Outlet + Sync> Closer for OutletCloser<O> {
    fn close(&self) {
        let outlet = self.0.clone();
        tokio::spawn(async move { outlet.close().await });
    }
}

pub(crate) struct SshServer {
    spawner: Arc<dyn Spawner + Send + Sync>,
    sessions: Sessions,
    diagram_dir: DiagramDir,
}

impl SshServer {
    pub(crate) fn new(spawner: Arc<dyn Spawner + Send + Sync>, diagram_dir: DiagramDir) -> Self {
        Self {
            spawner,
            sessions: Sessions::default(),
            diagram_dir,
        }
    }
}

impl Server for SshServer {
    type Handler = VisitorHandler;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> VisitorHandler {
        VisitorHandler::new(
            self.spawner.clone(),
            self.sessions.clone(),
            self.diagram_dir.clone(),
        )
    }
}

pub(crate) struct VisitorHandler {
    spawner: Arc<dyn Spawner + Send + Sync>,
    sessions: Sessions,
    diagram_dir: DiagramDir,
    fingerprint: Option<String>,
    registration: Option<(String, Token)>,
    window: Window,
    session: Option<Arc<dyn Session>>,
}

impl VisitorHandler {
    pub(crate) fn new(
        spawner: Arc<dyn Spawner + Send + Sync>,
        sessions: Sessions,
        diagram_dir: DiagramDir,
    ) -> Self {
        Self {
            spawner,
            sessions,
            diagram_dir,
            fingerprint: None,
            registration: None,
            window: Window::default(),
            session: None,
        }
    }

    fn record_key(&mut self, key: &PublicKey) {
        self.fingerprint = Some(key.fingerprint(HashAlg::Sha256).to_string());
    }

    fn start_shell(&mut self, outlet: impl Outlet, path: &str) -> std::io::Result<()> {
        let session: Arc<dyn Session> = Arc::from(self.spawner.spawn(self.window, path)?);
        tokio::spawn(relay(Arc::downgrade(&session), outlet));
        self.session = Some(session);
        Ok(())
    }

    async fn begin(&mut self, outlet: impl Outlet + Sync) -> std::io::Result<()> {
        let Some(fingerprint) = self.fingerprint.clone() else {
            outlet.send(no_key_message().into_bytes()).await;
            outlet.close().await;
            return Ok(());
        };
        self.sessions.take_over(&fingerprint);
        let path = self.diagram_dir.for_key(&fingerprint)?;
        self.start_shell(outlet.clone(), &path)?;
        let token = self
            .sessions
            .register(&fingerprint, Arc::new(OutletCloser(outlet)));
        self.registration = Some((fingerprint, token));
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

impl Drop for VisitorHandler {
    fn drop(&mut self) {
        if let Some((fingerprint, token)) = self.registration.take() {
            self.sessions.unregister(&fingerprint, token);
        }
    }
}

impl Handler for VisitorHandler {
    type Error = russh::Error;

    async fn auth_publickey(&mut self, _user: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        self.record_key(key);
        Ok(Auth::Accept)
    }

    async fn auth_keyboard_interactive<'a>(
        &'a mut self,
        _user: &str,
        _submethods: &str,
        _response: Option<Response<'a>>,
    ) -> Result<Auth, Self::Error> {
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
        self.begin(outlet).await?;
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
        spawner.expect_spawn().returning(move |_, _| {
            Ok(Box::new(session.lock().unwrap().take().unwrap()) as Box<dyn Session>)
        });
        Arc::new(spawner)
    }

    fn diagram_dir(name: &str) -> DiagramDir {
        DiagramDir {
            root: std::env::temp_dir()
                .join(format!("dre-visitor-{}-{name}", std::process::id()))
                .to_string_lossy()
                .into_owned(),
        }
    }

    fn handler_of(spawner: Arc<dyn Spawner + Send + Sync>) -> VisitorHandler {
        VisitorHandler::new(spawner, Sessions::default(), diagram_dir("unused"))
    }

    fn blocking_session() -> (MockSession, std::sync::mpsc::Sender<Vec<u8>>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut session = MockSession::new();
        session
            .expect_read()
            .returning(move || receiver.recv().ok());
        (session, sender)
    }

    fn keyed_handler(
        spawner: MockSpawner,
        sessions: &Sessions,
        name: &str,
        key: &PublicKey,
    ) -> VisitorHandler {
        let mut handler =
            VisitorHandler::new(Arc::new(spawner), sessions.clone(), diagram_dir(name));
        handler.record_key(key);
        handler
    }

    fn spawner_of_blocking_session() -> (MockSpawner, std::sync::mpsc::Sender<Vec<u8>>) {
        let (session, sender) = blocking_session();
        let session = Mutex::new(Some(session));
        let mut spawner = MockSpawner::new();
        spawner.expect_spawn().times(1).returning(move |_, _| {
            Ok(Box::new(session.lock().unwrap().take().unwrap()) as Box<dyn Session>)
        });
        (spawner, sender)
    }

    fn started_handler(session: MockSession) -> VisitorHandler {
        let mut handler = handler_of(spawner_of(session));
        handler.start_shell(fake_outlet().0, "diagram.dre").unwrap();
        handler
    }

    #[tokio::test]
    async fn any_public_key_is_accepted() {
        let mut handler = handler_of(Arc::new(MockSpawner::new()));
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
            .withf(|window, _| *window == SIZE)
            .times(1)
            .returning(|_, _| Ok(Box::new(silent_session())));
        let mut handler = handler_of(Arc::new(spawner));
        handler.window = SIZE;
        handler.start_shell(fake_outlet().0, "diagram.dre").unwrap();
    }

    #[tokio::test]
    async fn start_shell_spawns_with_the_given_path() {
        let mut spawner = MockSpawner::new();
        spawner
            .expect_spawn()
            .withf(|_, path| path == "some/diagram.dre")
            .times(1)
            .returning(|_, _| Ok(Box::new(silent_session())));
        let mut handler = handler_of(Arc::new(spawner));
        handler
            .start_shell(fake_outlet().0, "some/diagram.dre")
            .unwrap();
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
        let mut handler = handler_of(spawner_of(session));
        let (outlet, mut received) = fake_outlet();
        handler.start_shell(outlet, "diagram.dre").unwrap();
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
        let mut handler = handler_of(spawner_of(silent_session()));
        let (outlet, mut received) = fake_outlet();
        handler.start_shell(outlet, "diagram.dre").unwrap();
        assert_eq!(received.recv().await, Some(None));
    }

    #[tokio::test]
    async fn two_connections_get_two_separate_sessions() {
        let mut spawner = MockSpawner::new();
        spawner.expect_spawn().times(2).returning(|_, _| {
            let mut session = silent_session();
            session.expect_write().times(1).return_const(());
            Ok(Box::new(session))
        });
        let mut server = SshServer::new(Arc::new(spawner), diagram_dir("server"));
        let mut first = server.new_client(None);
        let mut second = server.new_client(None);
        first.start_shell(fake_outlet().0, "diagram.dre").unwrap();
        second.start_shell(fake_outlet().0, "diagram.dre").unwrap();
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
            .returning(move |_, _| Ok(Box::new(probe.lock().unwrap().take().unwrap())));
        let mut handler = handler_of(Arc::new(spawner));
        let (outlet, mut received) = fake_outlet();
        handler.start_shell(outlet, "diagram.dre").unwrap();
        received.recv().await;
        drop(handler);
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn auth_none_is_rejected() {
        let mut handler = handler_of(Arc::new(MockSpawner::new()));
        let auth = handler.auth_none("anyone").await.unwrap();
        assert!(matches!(auth, Auth::Reject { .. }));
    }

    #[tokio::test]
    async fn keyboard_interactive_is_accepted_without_a_fingerprint() {
        let mut handler = handler_of(Arc::new(MockSpawner::new()));
        let auth = handler
            .auth_keyboard_interactive("anyone", "", None)
            .await
            .unwrap();
        assert!(matches!(auth, Auth::Accept));
        assert_eq!(handler.fingerprint, None);
    }

    #[test]
    fn the_no_key_message_names_the_public_host() {
        assert!(no_key_message().contains(PUBLIC_HOST));
    }

    #[tokio::test]
    async fn a_keyless_visitor_gets_the_message_and_the_channel_closes_without_a_spawn() {
        let mut spawner = MockSpawner::new();
        spawner.expect_spawn().times(0);
        let mut handler = handler_of(Arc::new(spawner));
        let (outlet, mut received) = fake_outlet();
        handler.begin(outlet).await.unwrap();
        assert_eq!(
            received.recv().await,
            Some(Some(no_key_message().into_bytes()))
        );
        assert_eq!(received.recv().await, Some(None));
    }

    #[tokio::test]
    async fn a_key_spawns_with_the_path_of_its_diagram_dir() {
        let key = public_key();
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
        let directory = diagram_dir("path");
        let expected = directory.for_key(&fingerprint).unwrap();
        let mut spawner = MockSpawner::new();
        spawner
            .expect_spawn()
            .withf(move |_, path| path == expected)
            .times(1)
            .returning(|_, _| Ok(Box::new(silent_session())));
        let mut handler = keyed_handler(spawner, &Sessions::default(), "path", &key);
        handler.begin(fake_outlet().0).await.unwrap();
        std::fs::remove_dir_all(&directory.root).unwrap();
    }

    #[tokio::test]
    async fn the_same_key_connecting_twice_closes_the_first_and_spawns_a_second() {
        let key = public_key();
        let sessions = Sessions::default();
        let (first_spawner, _first_input) = spawner_of_blocking_session();
        let (second_spawner, _second_input) = spawner_of_blocking_session();
        let mut first = keyed_handler(first_spawner, &sessions, "same", &key);
        let mut second = keyed_handler(second_spawner, &sessions, "same", &key);
        let (first_outlet, mut first_received) = fake_outlet();
        let (second_outlet, mut second_received) = fake_outlet();
        first.begin(first_outlet).await.unwrap();
        second.begin(second_outlet).await.unwrap();
        assert_eq!(first_received.recv().await, Some(None));
        assert!(second_received.try_recv().is_err());
        std::fs::remove_dir_all(&first.diagram_dir.root).unwrap();
    }

    #[tokio::test]
    async fn different_keys_never_close_each_other() {
        let sessions = Sessions::default();
        let (first_spawner, _first_input) = spawner_of_blocking_session();
        let (second_spawner, _second_input) = spawner_of_blocking_session();
        let mut first = keyed_handler(first_spawner, &sessions, "different", &public_key());
        let mut second = keyed_handler(second_spawner, &sessions, "different", &public_key());
        let (first_outlet, mut first_received) = fake_outlet();
        first.begin(first_outlet).await.unwrap();
        second.begin(fake_outlet().0).await.unwrap();
        tokio::task::yield_now().await;
        assert!(first_received.try_recv().is_err());
        std::fs::remove_dir_all(&first.diagram_dir.root).unwrap();
    }

    #[tokio::test]
    async fn an_old_connection_ending_late_does_not_evict_its_replacement() {
        let key = public_key();
        let sessions = Sessions::default();
        let (first_spawner, _first_input) = spawner_of_blocking_session();
        let (second_spawner, _second_input) = spawner_of_blocking_session();
        let (third_spawner, _third_input) = spawner_of_blocking_session();
        let mut first = keyed_handler(first_spawner, &sessions, "late", &key);
        let mut second = keyed_handler(second_spawner, &sessions, "late", &key);
        let mut third = keyed_handler(third_spawner, &sessions, "late", &key);
        let (second_outlet, mut second_received) = fake_outlet();
        first.begin(fake_outlet().0).await.unwrap();
        second.begin(second_outlet).await.unwrap();
        drop(first);
        third.begin(fake_outlet().0).await.unwrap();
        assert_eq!(second_received.recv().await, Some(None));
        std::fs::remove_dir_all(&second.diagram_dir.root).unwrap();
    }
}
