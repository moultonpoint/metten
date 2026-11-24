use super::{
    budget::{Budget, Traffic},
    signing::Key,
    *,
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig},
};
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub struct Connection {
    socket: Option<Socket>,
    provider: Provider,
    fence: Fence,
    #[cfg(test)]
    generation: u64,
    retry: Arc<std::sync::Mutex<budget::Retry>>,
    _memory: tokio::sync::OwnedSemaphorePermit,
}
impl Connection {
    fn fail(&mut self) {
        self.fence.revoke();
        self.socket.take();
    }
    pub async fn close(&mut self, deadline: Instant) -> Result<(), Failure> {
        self.fence.revoke();
        if let Ok(mut retry) = self.retry.lock() {
            retry.delay(Failure::Stopped);
        }
        let mut socket = self.socket.take();
        if let Some(socket) = &mut socket {
            tokio::time::timeout_at(deadline, socket.close(None))
                .await
                .map_err(|_| Failure::Deadline)?
                .map_err(|_| Failure::Transport)
        } else {
            Ok(())
        }
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.fence.revoke();
        if let Ok(mut retry) = self.retry.lock() {
            retry.delay(Failure::Stopped);
        }
    }
}
// A borrowed public operation may be dropped by its caller's select/timeout.
struct InFlight<'a> {
    connection: &'a mut Connection,
    completed: bool,
    retry: &'a std::sync::Mutex<budget::Retry>,
}
impl InFlight<'_> {
    fn finish<T>(mut self, result: Result<T, Failure>) -> Result<T, Failure> {
        self.completed = true;
        if let Err(error) = result {
            self.connection.fail();
            self.retry
                .lock()
                .map_err(|_| Failure::Unavailable)?
                .delay(error);
        }
        result
    }
}
impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.connection.fail();
            if let Ok(mut retry) = self.retry.lock() {
                retry.delay(Failure::Stopped);
            }
        }
    }
}
pub struct Transport {
    environment: Environment,
    isolated: bool,
    client: reqwest::Client,
    budgets: [Arc<std::sync::Mutex<Budget>>; 3],
    memory: memory::Memory,
    retries: [Arc<std::sync::Mutex<budget::Retry>>; 3],
    http_slots: Arc<tokio::sync::Semaphore>,
    handshake_slots: [Arc<tokio::sync::Semaphore>; 3],
    #[cfg(test)]
    peer: Option<std::net::SocketAddr>,
}
impl Transport {
    pub(crate) fn isolated() -> Result<Self, Failure> {
        let mut t = Self::new(Environment::Demo)?;
        t.isolated = true;
        Ok(t)
    }
    pub fn environment(&self) -> Environment {
        self.environment
    }
    pub fn is_isolated(&self) -> bool {
        self.isolated
    }

    pub fn new(environment: Environment) -> Result<Self, Failure> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .pool_max_idle_per_host(1)
            .http1_only()
            .build()
            .map_err(|_| Failure::Transport)?;
        Ok(Self {
            environment,
            isolated: false,
            client,
            budgets: std::array::from_fn(|_| Arc::new(std::sync::Mutex::new(Budget::new()))),
            memory: memory::Memory::default(),
            retries: std::array::from_fn(|index| {
                Arc::new(std::sync::Mutex::new(budget::Retry::new(index as u64 + 1)))
            }),
            http_slots: Arc::new(tokio::sync::Semaphore::new(2)),
            handshake_slots: std::array::from_fn(|_| Arc::new(tokio::sync::Semaphore::new(1))),
            #[cfg(test)]
            peer: None,
        })
    }
    fn rest_url(&self, path: &str) -> String {
        #[cfg(test)]
        if let Some(peer) = self.peer {
            return format!("http://{peer}{path}");
        }
        format!("{}{path}", self.environment.rest())
    }
    /// Actual transport polls: [REST reads, WebSocket handshakes, control writes].
    #[cfg(test)]
    pub fn attempts(&self, provider: Provider) -> Result<[u64; 3], Failure> {
        Ok(self.budgets[provider.index()]
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .attempts)
    }
    #[cfg(test)]
    pub fn controlled(peer: std::net::SocketAddr) -> Self {
        let mut transport = Self::new(Environment::Demo).unwrap();
        transport.client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .unwrap();
        transport.peer = Some(peer);
        transport
    }
    pub fn memory(&self) -> &memory::Memory {
        &self.memory
    }
    pub fn counters(&self, provider: Provider) -> Result<[u64; 5], Failure> {
        let budget = self.budgets[provider.index()]
            .lock()
            .map_err(|_| Failure::Unavailable)?;
        Ok([
            budget.attempts[0],
            budget.attempts[1],
            budget.attempts[2],
            budget.pongs,
            budget.data_messages,
        ])
    }
    pub fn retry_usable(&self, provider: Provider, now: Instant) -> Result<(), Failure> {
        self.retries[provider.index()]
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .usable(now);
        Ok(())
    }
    pub fn slow_retry(&self, provider: Provider) -> Result<bool, Failure> {
        Ok(self.retries[provider.index()]
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .slow())
    }
    async fn start_attempt(
        &self,
        provider: Provider,
        permit: &mut budget::Permit,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
    ) -> Result<(), Failure> {
        loop {
            if *stop.borrow() {
                return Err(Failure::Stopped);
            }
            let delay = {
                let mut retry = self.retries[provider.index()]
                    .lock()
                    .map_err(|_| Failure::Unavailable)?;
                match retry.claim(Instant::now()) {
                    Some(delay) => Some(delay),
                    None => {
                        permit.start()?;
                        None
                    }
                }
            };
            match delay {
                Some(delay) => wait(stop, deadline, delay).await?,
                None => return Ok(()),
            }
        }
    }
    async fn permit(
        &self,
        provider: Provider,
        traffic: Traffic,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
    ) -> Result<budget::Permit, Failure> {
        loop {
            if *stop.borrow() {
                return Err(Failure::Stopped);
            }
            let result =
                budget::Permit::reserve(self.budgets[provider.index()].clone(), traffic, deadline);
            match result {
                Err(Failure::Budget) => wait(stop, deadline, Duration::from_millis(100)).await?,
                result => return result,
            }
        }
    }
    pub async fn get(
        &self,
        route: &Route,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
        fence: &Fence,
    ) -> Result<memory::Bytes, Failure> {
        if self.isolated {
            return Err(Failure::InvalidRequest);
        }
        let deadline = deadline.min(Instant::now() + Duration::from_secs(10));
        fence
            .run(deadline, self.get_once_owned(route, stop, deadline, fence))
            .await
    }
    pub async fn connect(
        &self,
        provider: Provider,
        credential: Option<(&str, &Key)>,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
        fence: &Fence,
    ) -> Result<Connection, Failure> {
        if self.isolated {
            return Err(Failure::InvalidRequest);
        }
        let deadline = deadline.min(Instant::now() + Duration::from_secs(10));
        let work = async {
            loop {
                match self
                    .connect_once(provider, credential, stop, deadline, fence)
                    .await
                {
                    Ok(connection) => return Ok(connection),
                    Err(error) => match self.retries[provider.index()]
                        .lock()
                        .map_err(|_| Failure::Unavailable)?
                        .delay(error)
                    {
                        Some(_) => continue,
                        None => return Err(error),
                    },
                }
            }
        };
        fence.run(deadline, work).await
    }
    async fn get_once_owned(
        &self,
        route: &Route,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
        fence: &Fence,
    ) -> Result<memory::Bytes, Failure> {
        if matches!(route, Route::Socket) {
            return Err(Failure::InvalidRequest);
        }
        let path = route.path()?;
        let generation = fence.token();
        if !fence.current(generation) {
            return Err(Failure::Stopped);
        }
        let _slot = bounded(stop, deadline, async {
            self.http_slots
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| Failure::Stopped)
        })
        .await?;
        loop {
            if !fence.current(generation) {
                return Err(Failure::Stopped);
            }
            let mut permit = self
                .permit(Provider::Kalshi, Traffic::Read, stop, deadline)
                .await?;
            let charge = self.memory.reserve(MAX_BYTES)?;
            let mut admission_stop = stop.clone();
            let result = bounded(stop, deadline, async {
                self.start_attempt(Provider::Kalshi, &mut permit, &mut admission_stop, deadline)
                    .await?;
                read_body(&self.client, &self.rest_url(&path), charge).await
            })
            .await;
            if !fence.current(generation) {
                return Err(Failure::Stopped);
            }
            match result {
                Ok(bytes) => return Ok(bytes),
                Err(error) => match self.retries[Provider::Kalshi.index()]
                    .lock()
                    .map_err(|_| Failure::Unavailable)?
                    .delay(error)
                {
                    Some(_) => continue,
                    None => return Err(error),
                },
            }
        }
    }

    async fn connect_once(
        &self,
        provider: Provider,
        credential: Option<(&str, &Key)>,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
        fence: &Fence,
    ) -> Result<Connection, Failure> {
        let url = match provider {
            Provider::Kalshi => self.environment.socket(),
            Provider::Coinbase => "wss://ws-feed.exchange.coinbase.com",
            Provider::Kraken => "wss://ws.kraken.com/v2",
        };
        let generation = fence.token();
        if !fence.current(generation) {
            return Err(Failure::Stopped);
        }
        let _slot = bounded(stop, deadline, async {
            self.handshake_slots[provider.index()]
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| Failure::Stopped)
        })
        .await?;
        let mut permit = self
            .permit(provider, Traffic::Handshake, stop, deadline)
            .await?;
        #[cfg(test)]
        let local = self.peer.map(|peer| format!("ws://{peer}/trade-api/ws/v2"));
        #[cfg(test)]
        let url = local.as_deref().unwrap_or(url);
        let mut request = url
            .into_client_request()
            .map_err(|_| Failure::InvalidRequest)?;
        if provider == Provider::Kalshi {
            let (id, key) = credential.ok_or(Failure::Missing)?;
            if !super::super::provider_credential::valid_id(id) {
                return Err(Failure::InvalidCredential);
            }
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Failure::Signing)?
                .as_millis();
            let timestamp = u64::try_from(timestamp).map_err(|_| Failure::Signing)?;
            for (name, value) in [
                ("KALSHI-ACCESS-KEY", id.to_owned()),
                ("KALSHI-ACCESS-TIMESTAMP", timestamp.to_string()),
                (
                    "KALSHI-ACCESS-SIGNATURE",
                    key.sign(&Route::Socket, timestamp)?,
                ),
            ] {
                let mut value = value
                    .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
                    .map_err(|_| Failure::Signing)?;
                value.set_sensitive(true);
                request.headers_mut().insert(name, value);
            }
        } else if credential.is_some() {
            return Err(Failure::InvalidRequest);
        }

        // Frame + fragmented message + write buffer + 1 MiB protocol/read headroom.
        let memory = self.memory.reserve(4 * MAX_BYTES)?;
        let mut admission_stop = stop.clone();
        let (socket, _) = bounded(stop, deadline, async {
            self.start_attempt(provider, &mut permit, &mut admission_stop, deadline)
                .await?;
            tokio_tungstenite::connect_async_with_config(request, Some(config()), false)
                .await
                .map_err(websocket_failure)
        })
        .await?;
        if !fence.current(generation) {
            return Err(Failure::Stopped);
        }
        Ok(Connection {
            socket: Some(socket),
            provider,
            fence: fence.clone(),
            #[cfg(test)]
            generation,
            retry: self.retries[provider.index()].clone(),
            _memory: memory,
        })
    }
}
pub fn config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MAX_BYTES))
        .max_frame_size(Some(MAX_BYTES))
        .write_buffer_size(0)
        .max_write_buffer_size(MAX_BYTES + 1024)
}
async fn read_body(
    client: &reqwest::Client,
    url: &str,
    charge: tokio::sync::OwnedSemaphorePermit,
) -> Result<memory::Bytes, Failure> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| Failure::Transport)?;
    if response.status().is_redirection() {
        return Err(Failure::Protocol);
    }
    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Err(Failure::Denied);
    }
    if !response.status().is_success() {
        return Err(
            if response.status().is_server_error()
                || matches!(response.status().as_u16(), 408 | 429)
            {
                Failure::Transport
            } else {
                Failure::Protocol
            },
        );
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_BYTES as u64)
    {
        return Err(Failure::Capacity);
    }
    let mut bytes = Vec::with_capacity(MAX_BYTES);
    while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Transport)? {
        if chunk.len() > MAX_BYTES - bytes.len() {
            return Err(Failure::Capacity);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(memory::Bytes {
        data: bytes,
        _charge: charge,
    })
}
// Controlled peers use the same framing/body path, never a runtime-selected host.
#[cfg(test)]
pub async fn test_body(url: &str) -> Result<memory::Bytes, Failure> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build()
        .unwrap();
    read_body(&client, url, memory::Memory::default().reserve(MAX_BYTES)?).await
}
#[cfg(test)]
impl Transport {
    pub async fn send(
        &self,
        connection: &mut Connection,
        command: &Control,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
    ) -> Result<(), Failure> {
        let text = command.encode(connection.provider)?;
        if !connection.fence.current(connection.generation) {
            connection.fail();
            return Err(Failure::Stopped);
        }
        let retry = &self.retries[connection.provider.index()];
        let operation = InFlight {
            connection,
            completed: false,
            retry,
        };
        let connection = &mut *operation.connection;
        let fence = connection.fence.clone();
        let result = fence
            .run(deadline, async {
                let mut permit = self
                    .permit(connection.provider, Traffic::Control, stop, deadline)
                    .await?;
                let socket = connection.socket.as_mut().ok_or(Failure::Stopped)?;
                send_control(socket, text, &mut permit, stop, deadline).await
            })
            .await;
        operation.finish(result)
    }
    pub async fn receive(
        &self,
        connection: &mut Connection,
        stop: &mut watch::Receiver<bool>,
        deadline: Instant,
    ) -> Result<memory::Bytes, Failure> {
        let retry = &self.retries[connection.provider.index()];
        let operation = InFlight {
            connection,
            completed: false,
            retry,
        };
        let charge = self.memory.reserve(MAX_BYTES)?;
        let connection = &mut *operation.connection;
        let fence = connection.fence.clone();
        let result = fence
            .run(deadline, async {
                loop {
                    if !connection.fence.current(connection.generation) {
                        return Err(Failure::Stopped);
                    }
                    let socket = connection.socket.as_mut().ok_or(Failure::Stopped)?;
                    let message = bounded(stop, deadline, async {
                        socket
                            .next()
                            .await
                            .ok_or(Failure::Transport)?
                            .map_err(websocket_failure)
                    })
                    .await?;
                    match message {
                        Message::Text(text) => {
                            let mut budget = self.budgets[connection.provider.index()]
                                .lock()
                                .map_err(|_| Failure::Unavailable)?;
                            budget.data_messages = budget
                                .data_messages
                                .checked_add(1)
                                .ok_or(Failure::Capacity)?;
                            return self.memory.finish_receive(text.as_bytes().to_vec(), charge);
                        }
                        Message::Ping(_) => {
                            let mut permit = self
                                .permit(connection.provider, Traffic::KeepAlive, stop, deadline)
                                .await?;
                            bounded(stop, deadline, async {
                                permit.start()?;
                                socket.flush().await.map_err(websocket_failure)
                            })
                            .await?;
                        }
                        Message::Pong(_) => (),
                        Message::Close(_) => return Err(Failure::Transport),
                        _ => return Err(Failure::Protocol),
                    }
                }
            })
            .await;
        operation.finish(result)
    }
}

#[cfg(test)]
pub async fn local_connection(
    address: std::net::SocketAddr,
    provider: Provider,
    fence: Fence,
) -> Connection {
    let memory = memory::Memory::default();
    let charge = memory.reserve(4 * MAX_BYTES).unwrap();
    let (socket, _) = tokio_tungstenite::connect_async_with_config(
        format!("ws://{address}"),
        Some(config()),
        false,
    )
    .await
    .unwrap();
    let generation = fence.token();
    Connection {
        socket: Some(socket),
        provider,
        fence,
        generation,
        retry: Arc::new(std::sync::Mutex::new(budget::Retry::new(1))),
        _memory: charge,
    }
}

fn websocket_failure(error: tokio_tungstenite::tungstenite::Error) -> Failure {
    use tokio_tungstenite::tungstenite::Error;
    match error {
        Error::Http(response) if matches!(response.status().as_u16(), 401 | 403) => Failure::Denied,
        Error::Http(response)
            if response.status().is_server_error()
                || matches!(response.status().as_u16(), 408 | 429) =>
        {
            Failure::Transport
        }
        Error::Io(_)
        | Error::ConnectionClosed
        | Error::Protocol(
            tokio_tungstenite::tungstenite::error::ProtocolError::ResetWithoutClosingHandshake,
        ) => Failure::Transport,
        _ => Failure::Protocol,
    }
}

#[cfg(test)]
pub(super) async fn send_control<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
    text: String,
    permit: &mut budget::Permit,
    stop: &mut watch::Receiver<bool>,
    deadline: Instant,
) -> Result<(), Failure> {
    bounded(stop, deadline, async {
        permit.start()?;
        socket
            .send(Message::Text(text.into()))
            .await
            .map_err(websocket_failure)
    })
    .await
}

impl Transport {
    /// One owned provider future services both halves; normal control wakeups never cancel reads.
    pub(super) async fn serve(
        &self,
        mut connection: Connection,
        controls: &mut mpsc::Receiver<Control>,
        intake: mpsc::Sender<Intake>,
        tokens: [metten_engine::sources::StreamToken; 6],
        mut stop: watch::Receiver<bool>,
        progress: Option<watch::Sender<Progress>>,
    ) -> Result<(), Failure> {
        if let Some(progress) = &progress {
            progress.send_modify(|state| {
                for (i, token) in tokens.iter().enumerate() {
                    state.tokens[i].get_or_insert(*token);
                }
            });
        }
        let provider = connection.provider;
        let fence = connection.fence.clone();
        let operation = InFlight {
            connection: &mut connection,
            completed: false,
            retry: &self.retries[provider.index()],
        };
        let socket = operation.connection.socket.take().ok_or(Failure::Stopped)?;
        let (mut writer, mut reader) = socket.split();
        let (pong_tx, mut pong_rx) = mpsc::channel(1);
        let mut read_progress = progress.as_ref().map(|p| p.subscribe());
        let mut read_stop = stop.clone();
        let read_fence = fence.clone();
        let io_timeout = std::sync::atomic::AtomicBool::new(false);
        let mut credited = 0;
        let read = async {
            loop {
                let deadline = Instant::now() + Duration::from_secs(10);
                let charge = self.memory.reserve(MAX_BYTES)?;
                let mut receipt_tokens = tokens;
                if let Some(progress) = &progress {
                    for (i, token) in progress.borrow().tokens.iter().enumerate() {
                        if let Some(token) = token {
                            receipt_tokens[i] = *token;
                        }
                    }
                }
                let message = read_fence
                    .run(
                        deadline,
                        bounded(&mut read_stop, deadline, async {
                            let receive=reader.next();tokio::pin!(receive);
                            loop {
                                let timed=read_progress.as_ref().and_then(|p|p.borrow().deadline()).unwrap_or(deadline).min(deadline);
                                let changed=async {if let Some(p)=&mut read_progress{p.changed().await.map_err(|_|Failure::Stopped)?;Ok(p.borrow().priced)}else{std::future::pending::<Result<u64,Failure>>().await}};
                                tokio::select!{biased;
                                    _=tokio::time::sleep_until(timed)=>return Err(Failure::Deadline),
                                    change=changed=>{let priced=change?;if priced!=credited{credited=priced;self.retry_usable(provider,Instant::now())?;}},
                                    message=&mut receive=>break message.ok_or(Failure::Transport)?.map_err(websocket_failure),
                                }
                            }
                        }),
                    )
                    .await.inspect_err(|&error| {
                        if error==Failure::Deadline {io_timeout.store(true,Ordering::SeqCst);}
                    })?;
                match message {
                    Message::Text(text) => {
                        {
                            let mut budget = self.budgets[provider.index()]
                                .lock()
                                .map_err(|_| Failure::Unavailable)?;
                            budget.data_messages = budget
                                .data_messages
                                .checked_add(1)
                                .ok_or(Failure::Capacity)?;
                        }
                        let mut item = Intake::new(
                            provider,
                            &read_fence,
                            self.memory
                                .finish_receive(text.as_bytes().to_vec(), charge)?,
                            Instant::now(),
                        );
                        item.source_tokens = Some(receipt_tokens);
                        item.progress = progress.clone();
                        if item.bytes.len() > MAX_BYTES {
                            return Err(Failure::Capacity);
                        }
                        let deadline = item.received_at + MAX_INTAKE_AGE;
                        let permit = read_fence
                            .run(
                                deadline,
                                bounded(&mut read_stop, deadline, async {
                                    intake.reserve().await.map_err(|_| Failure::Stopped)
                                }),
                            )
                            .await?;
                        if !read_fence.current(item.generation) {
                            return Err(Failure::Stopped);
                        }
                        permit.send(item);
                    }
                    Message::Ping(_) => {
                        match pong_tx.try_send(()) {
                            Ok(()) | Err(mpsc::error::TrySendError::Full(())) => (), // One wake suffices; the protocol owns the pending pong.
                            Err(mpsc::error::TrySendError::Closed(())) => {
                                return Err(Failure::Stopped);
                            }
                        }
                    }
                    Message::Pong(_) => (),
                    Message::Close(_) => return Err(Failure::Transport),
                    _ => return Err(Failure::Protocol),
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), Failure>(())
        };
        let write_fence = fence.clone();
        let mut recovery_changes = progress.as_ref().map(|p| p.subscribe());
        let write = async {
            loop {
                if *stop.borrow() {
                    return Err(Failure::Stopped);
                }
                let desired = progress
                    .as_ref()
                    .and_then(|p| p.borrow().recovery.iter().flatten().next().cloned());
                let (recovery, command) = if pong_rx.try_recv().is_ok() {
                    (false, None)
                } else if let Some(command) = desired {
                    (true, Some(command))
                } else {
                    tokio::select! {biased;
                        _=stop.changed()=>return Err(Failure::Stopped),
                        pong=pong_rx.recv()=>{if pong.is_none(){return Err(Failure::Stopped)}(false,None)},
                        changed=async {if let Some(p)=&mut recovery_changes {p.changed().await.map_err(|_|Failure::Stopped)} else {std::future::pending::<Result<(),Failure>>().await}}=>{changed?;continue},
                        command=controls.recv()=>(false,Some(command.ok_or(Failure::Stopped)?)),
                    }
                };
                if let Some(Control::KalshiBookUpdate {
                    token: Some(token),
                    action: BookAction::Snapshot,
                    ..
                }) = &command
                    && progress.as_ref().is_some_and(|p| {
                        p.borrow()
                            .desired_book
                            .is_some_and(|desired| desired != *token)
                    })
                {
                    continue;
                }
                let text = command
                    .as_ref()
                    .map(|command| command.encode(provider))
                    .transpose()?;
                let deadline = Instant::now() + Duration::from_secs(5);
                let traffic = if command.is_some() {
                    Traffic::Control
                } else {
                    Traffic::KeepAlive
                };
                let mut permit = self.permit(provider, traffic, &mut stop, deadline).await?;
                let attempted = AtomicBool::new(false);
                write_fence
                    .run(
                        deadline,
                        bounded(&mut stop, deadline, async {
                            let mut admitted = Ok(false);
                            if let (Some(progress), Some(command)) = (&progress, &command) {
                                progress.send_modify(|p| {
                                    if !recovery
                                        || p.recovery.iter().flatten().any(|c| c == command)
                                    {
                                        admitted = p
                                            .request(command)
                                            .and_then(|()| permit.start())
                                            .map(|()| true);
                                        if admitted == Ok(true) {
                                            for pending in &mut p.recovery {
                                                if pending.as_ref() == Some(command) {
                                                    *pending = None;
                                                }
                                            }
                                        }
                                    }
                                });
                            } else {
                                admitted = permit.start().map(|_| true);
                            }
                            if !admitted? {
                                return Ok(());
                            }
                            attempted.store(true, Ordering::SeqCst);
                            if let Some(command) = text {
                                writer
                                    .send(Message::Text(command.into()))
                                    .await
                                    .map_err(websocket_failure)
                            } else {
                                writer.flush().await.map_err(websocket_failure)
                            }
                        }),
                    )
                    .await
                    .inspect_err(|&error| {
                        if error == Failure::Deadline && attempted.load(Ordering::SeqCst) {
                            io_timeout.store(true, Ordering::SeqCst);
                        }
                    })?;
            }
            #[allow(unreachable_code)]
            Ok::<(), Failure>(())
        };
        let result = tokio::try_join!(read, write).map(|_| ());
        // Both child futures are gone before close; neither can outlive this provider slot.
        let socket = writer.reunite(reader).map_err(|_| Failure::Unavailable)?;
        operation.connection.socket = Some(socket);
        if matches!(result, Err(Failure::Stopped)) {
            let _ = operation.connection.close(Instant::now() + SHUTDOWN).await;
        }
        let timed_out = io_timeout.load(Ordering::SeqCst) && result == Err(Failure::Deadline);
        if timed_out {
            // Established I/O actually timed out; a control-permit expiry alone never enters this branch.
            operation
                .finish::<()>(Err(Failure::Transport))
                .map_err(|error| {
                    if error == Failure::Transport {
                        Failure::Deadline
                    } else {
                        error
                    }
                })
        } else {
            operation.finish(result)
        }
    }
}

/// Transport wait deadlines and a count of frames carrying an engine-accepted price value, which only
/// paces reconnects. These grant no freshness or history capability.
#[derive(Clone)]
pub(super) struct Progress {
    requested: [Option<Instant>; 6],
    pub acknowledged: [bool; 6],
    data: [bool; 6],
    books: [Option<String>; 2],
    book_data: [bool; 2],
    pending_ids: [Option<u32>; 6],
    unsubscribe_ids: [Option<(u32, Vec<u32>)>; 4],
    completed_unsubscribe_ids: [Option<u32>; 4],
    pub recovery: [Option<Control>; 3],
    disabled: [bool; 6],
    pub tokens: [Option<metten_engine::sources::StreamToken>; 6],
    pub desired_book: Option<metten_engine::sources::StreamToken>,
    pub priced: u64,
}
impl Progress {
    pub fn new() -> Self {
        Self {
            requested: [None; 6],
            acknowledged: [false; 6],
            data: [false; 6],
            books: [None, None],
            book_data: [false; 2],
            pending_ids: [None; 6],
            unsubscribe_ids: [const { None }; 4],
            completed_unsubscribe_ids: [None; 4],
            recovery: [None, None, None],
            disabled: [false; 6],
            tokens: [None; 6],
            desired_book: None,
            priced: 0,
        }
    }
    fn request(&mut self, command: &Control) -> Result<(), Failure> {
        if let Control::KalshiUnsubscribe { id, subscriptions } = command {
            if subscriptions.is_empty() || subscriptions.len() > 4 {
                return Err(Failure::Capacity);
            }
            let slot = self
                .unsubscribe_ids
                .iter_mut()
                .find(|pending| pending.is_none())
                .ok_or(Failure::Capacity)?;
            *slot = Some((*id, subscriptions.clone()));
            return Ok(());
        }
        use metten_engine::sources::Stream;
        let stream = match command {
            Control::Coinbase { subscribe: true } => Stream::Coinbase,
            Control::Kraken { subscribe: true } => Stream::Kraken,
            Control::KalshiSubscribe { channel, .. } => match channel {
                Channel::Book => Stream::Book,
                Channel::Trade => Stream::Trades,
                Channel::Brti => Stream::Brti,
                Channel::Lifecycle => Stream::Metadata,
            },
            Control::KalshiBookUpdate {
                action: BookAction::Snapshot | BookAction::Add,
                ..
            } => Stream::Book,
            Control::KalshiTradeUpdate {
                action: BookAction::Add,
                ..
            } => Stream::Trades,
            Control::KalshiIndexes { .. } => Stream::Brti,
            _ => return Ok(()),
        };
        if let Control::KalshiTradeUpdate { token, .. } = command {
            self.tokens[Stream::Trades as usize] = Some(*token);
        }
        if let Control::KalshiBookUpdate {
            token: Some(token), ..
        } = command
        {
            self.tokens[Stream::Book as usize] = Some(*token);
        }
        let i = stream as usize;
        self.disabled[i] = false;
        if let Control::KalshiIndexes {
            token: Some(token), ..
        } = command
        {
            self.tokens[i] = Some(*token);
        }
        self.pending_ids[i] = match command {
            Control::KalshiSubscribe { id, .. }
            | Control::KalshiBookUpdate { id, .. }
            | Control::KalshiTradeUpdate { id, .. }
            | Control::KalshiIndexes { id, .. } => Some(*id),
            _ => None,
        };
        self.requested[i] = Some(Instant::now());
        self.data[i] = stream == Stream::Metadata;
        self.acknowledged[i] = false;
        if stream == Stream::Book
            && let Control::KalshiSubscribe { markets, .. }
            | Control::KalshiBookUpdate { markets, .. } = command
        {
            self.books = std::array::from_fn(|i| markets.get(i).cloned());
            self.book_data = [false; 2];
        }
        Ok(())
    }
    pub fn disable(&mut self, stream: metten_engine::sources::Stream) {
        self.requested[stream as usize] = None;
        self.disabled[stream as usize] = true;
        let slot = match stream {
            metten_engine::sources::Stream::Book => Some(0),
            metten_engine::sources::Stream::Trades => Some(1),
            metten_engine::sources::Stream::Brti => Some(2),
            _ => None,
        };
        if let Some(slot) = slot {
            self.recovery[slot] = None;
        }
    }
    pub fn recover(&mut self, control: Control) {
        let slot = match &control {
            Control::KalshiBookUpdate { .. } => 0,
            Control::KalshiTradeUpdate { .. } => 1,
            Control::KalshiIndexes { .. } => 2,
            _ => return,
        };
        self.recovery[slot] = Some(control);
    }
    pub fn requested_stream(&self, id: u32) -> Option<metten_engine::sources::Stream> {
        self.pending_ids
            .iter()
            .position(|pending| *pending == Some(id))
            .map(|i| super::source::SOURCE_STREAMS[i])
    }
    pub fn unsubscribed(&mut self, sid: u32) {
        for slot in &mut self.unsubscribe_ids {
            if let Some((id, subscriptions)) = slot {
                subscriptions.retain(|pending| *pending != sid);
                if subscriptions.is_empty() {
                    // Keep bounded command history for a code-7 reply racing this completion.
                    let vacant = self
                        .completed_unsubscribe_ids
                        .iter()
                        .position(Option::is_none)
                        .unwrap_or(self.completed_unsubscribe_ids.len() - 1);
                    self.completed_unsubscribe_ids[..=vacant].rotate_right(1);
                    self.completed_unsubscribe_ids[0] = Some(*id);
                    *slot = None;
                }
            }
        }
    }
    pub fn complete_unsubscribe(&mut self, id: u32) -> bool {
        if let Some(slot) = self
            .unsubscribe_ids
            .iter_mut()
            .find(|pending| pending.as_ref().is_some_and(|(pending, _)| *pending == id))
        {
            *slot = None;
            true
        } else if let Some(slot) = self
            .completed_unsubscribe_ids
            .iter_mut()
            .find(|pending| **pending == Some(id))
        {
            *slot = None;
            true
        } else {
            false
        }
    }
    pub fn acknowledge(&mut self, id: u32) {
        self.complete_unsubscribe(id);
        for (i, pending) in self.pending_ids.iter().enumerate() {
            if *pending == Some(id) {
                self.acknowledged[i] = true;
            }
        }
    }
    pub fn received(&mut self, stream: metten_engine::sources::Stream, market: Option<&str>) {
        self.acknowledged[stream as usize] = true;
        if stream == metten_engine::sources::Stream::Book {
            for (i, ticker) in self.books.iter().enumerate() {
                if ticker.as_deref() == market {
                    self.book_data[i] = true
                }
            }
            self.data[stream as usize] = self
                .books
                .iter()
                .enumerate()
                .all(|(i, t)| t.is_none() || self.book_data[i]);
        } else {
            self.data[stream as usize] = true
        }
    }
    fn deadline(&self) -> Option<Instant> {
        self.requested
            .iter()
            .enumerate()
            .filter_map(|(i, start)| {
                start.and_then(|start| {
                    if !self.acknowledged[i] {
                        Some(start + Duration::from_secs(5))
                    } else if !self.data[i] {
                        Some(start + Duration::from_secs(10))
                    } else {
                        None
                    }
                })
            })
            .min()
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    #[test]
    fn socket_loss_is_transient_but_invalid_frames_are_not() {
        use tokio_tungstenite::tungstenite::{Error, error::ProtocolError};
        for error in [
            Error::Io(std::io::ErrorKind::BrokenPipe.into()),
            Error::ConnectionClosed,
            Error::Protocol(ProtocolError::ResetWithoutClosingHandshake),
        ] {
            assert_eq!(websocket_failure(error), Failure::Transport);
        }
        assert_eq!(
            websocket_failure(Error::Protocol(ProtocolError::InvalidOpcode(3))),
            Failure::Protocol
        );
    }
    #[test]
    fn acknowledgment_and_both_replacement_books_have_independent_deadlines() {
        let mut p = Progress::new();
        for id in 1..=4 {
            p.request(&Control::KalshiUnsubscribe {
                id,
                subscriptions: vec![id],
            })
            .unwrap();
        }
        assert_eq!(
            p.request(&Control::KalshiUnsubscribe {
                id: 5,
                subscriptions: vec![5]
            }),
            Err(Failure::Capacity)
        );
        for id in 1..=4 {
            p.acknowledge(id);
        }
        assert!(p.unsubscribe_ids.iter().all(Option::is_none));
        p.request(&Control::KalshiSubscribe {
            id: 1,
            channel: Channel::Book,
            markets: vec!["KXBTC15M-M".into(), "KXBTC15M-N".into()],
        })
        .unwrap();
        let start = p.requested[0].unwrap();
        assert_eq!(p.deadline(), Some(start + Duration::from_secs(5)));
        p.acknowledge(1);
        assert_eq!(p.deadline(), Some(start + Duration::from_secs(10)));
        p.received(metten_engine::sources::Stream::Book, Some("KXBTC15M-M"));
        assert!(p.deadline().is_some());
        p.received(metten_engine::sources::Stream::Book, Some("KXBTC15M-N"));
        assert!(p.deadline().is_none());
        for id in 1..10000 {
            p.recover(Control::KalshiIndexes {
                id,
                sid: 3,
                token: None,
            });
        }
        assert_eq!(p.recovery.iter().flatten().count(), 1);
        assert!(matches!(
            p.recovery[2],
            Some(Control::KalshiIndexes { id: 9999, .. })
        ));
    }
    #[tokio::test(start_paused = true)]
    async fn explicit_close_and_unpolled_owned_drop_restart_the_usable_interval() {
        let transport = Transport::new(Environment::Demo).unwrap();
        let strategy = metten_engine::strategy::ValidatedStrategy::parse(include_str!(
            "../../../examples/learning.json"
        ))
        .unwrap();
        let worker = SourceWorker::prepare(&strategy, &[], transport.memory().clone()).unwrap();
        for close in [true, false] {
            let retry = transport.retries[Provider::Coinbase.index()].clone();
            {
                let mut state = retry.lock().unwrap();
                *state = budget::Retry::new(1);
                for _ in 0..5 {
                    state.delay(Failure::Transport);
                }
                state.usable(Instant::now());
            }
            let fence = Fence::new();
            let mut connection = Connection {
                socket: None,
                provider: Provider::Coinbase,
                generation: fence.token(),
                fence: fence.clone(),
                retry: retry.clone(),
                _memory: transport.memory.reserve(4 * MAX_BYTES).unwrap(),
            };
            if close {
                connection.close(Instant::now() + SHUTDOWN).await.unwrap();
            } else {
                let (_tx, mut controls) = mpsc::channel(1);
                let (intake, _) = intake();
                let (_stop, stop) = watch::channel(false);
                drop(transport.serve(
                    connection,
                    &mut controls,
                    intake,
                    worker.tokens(),
                    stop,
                    None,
                ));
            }
            assert!(!fence.current(fence.token()));
            tokio::time::advance(Duration::from_secs(30)).await;
            retry.lock().unwrap().usable(Instant::now());
            assert!(
                retry.lock().unwrap().slow(),
                "the disconnected interval cannot count as usable data"
            );
        }
    }
    #[test]
    fn idless_unsubscribe_completion_reuses_slots_and_preserves_other_commands() {
        let mut p = Progress::new();
        p.request(&Control::KalshiUnsubscribe {
            id: 100,
            subscriptions: vec![10, 11],
        })
        .unwrap();
        for id in 1..=12 {
            p.request(&Control::KalshiUnsubscribe {
                id,
                subscriptions: vec![3],
            })
            .unwrap();
            p.unsubscribed(3);
            assert!(p.complete_unsubscribe(id));
            assert!(!p.complete_unsubscribe(id));
            assert_eq!(p.unsubscribe_ids.iter().flatten().count(), 1);
        }
        p.unsubscribed(10);
        assert_eq!(p.unsubscribe_ids.iter().flatten().count(), 1);
        p.unsubscribed(11);
        assert!(p.unsubscribe_ids.iter().all(Option::is_none));
        assert!(p.complete_unsubscribe(100));
        assert!(!p.complete_unsubscribe(999));
        for id in 100..=104 {
            p.request(&Control::KalshiUnsubscribe {
                id,
                subscriptions: vec![3],
            })
            .unwrap();
            p.unsubscribed(3);
            if id == 101 {
                assert!(p.complete_unsubscribe(id));
            }
        }
        assert!(p.complete_unsubscribe(100));
        for id in 20..=31 {
            p.request(&Control::KalshiUnsubscribe {
                id,
                subscriptions: vec![3],
            })
            .unwrap();
            p.unsubscribed(3);
        }
        assert!(p.unsubscribe_ids.iter().all(Option::is_none));
        assert_eq!(p.completed_unsubscribe_ids.iter().flatten().count(), 4);
    }
    #[test]
    fn provider_ending_racing_with_unsubscribe_keeps_the_other_stream() {
        for completion_first in [false, true] {
            let memory = memory::Memory::default();
            let mut worker = SourceWorker::prepare(
                &super::super::tests::all_source_strategy(),
                &super::super::tests::source_markets(),
                memory.clone(),
            )
            .unwrap();
            let fence = Fence::new();
            worker
                .connection(
                    Provider::Kalshi,
                    worker.tokens()[0],
                    metten_engine::sources::Connection::Open,
                    0,
                )
                .unwrap();
            let (outbound, _controls) = mpsc::channel(7);
            worker.controls[0] = Some(outbound);
            let progress = watch::channel(Progress::new()).0;
            let feed = |worker: &mut SourceWorker, bytes: &[u8]| {
                let mut item = Intake::new(
                    Provider::Kalshi,
                    &fence,
                    memory.copy(bytes).unwrap(),
                    Instant::now(),
                );
                item.source_tokens = Some(worker.tokens());
                item.receipt_ms = Some(0);
                item.progress = Some(progress.clone());
                worker.process(item, 0, 0, |_| Ok(()))
            };
            for (sid, channel) in [(1, "orderbook_delta"), (3, "cfbenchmarks_value")] {
                feed(
                    &mut worker,
                    format!(
                        r#"{{"type":"subscribed","msg":{{"sid":{sid},"channel":"{channel}"}}}}"#
                    )
                    .as_bytes(),
                )
                .unwrap();
            }
            worker.membership([true, false, true, true], 0).unwrap();
            progress.send_modify(|p| {
                p.request(&Control::KalshiUnsubscribe {
                    id: 55,
                    subscriptions: vec![3],
                })
                .unwrap()
            });
            feed(
                &mut worker,
                br#"{"type":"error","sid":3,"seq":1,"msg":{"code":10}}"#,
            )
            .unwrap();
            let completed = br#"{"type":"unsubscribed","sid":3,"seq":2}"#.as_slice();
            let missing = br#"{"type":"error","id":55,"msg":{"code":7}}"#.as_slice();
            for frame in if completion_first {
                [completed, missing]
            } else {
                [missing, completed]
            } {
                feed(&mut worker, frame).unwrap();
                if frame == completed {
                    assert!(
                        progress
                            .borrow()
                            .unsubscribe_ids
                            .iter()
                            .all(Option::is_none)
                    );
                }
                assert!(fence.current(fence.token()));
                assert!(
                    worker
                        .snapshot(0)
                        .unwrap()
                        .stream_current(metten_engine::sources::Stream::Book)
                );
            }
            assert_eq!(
                feed(
                    &mut worker,
                    br#"{"type":"error","id":999,"msg":{"code":7}}"#
                ),
                Err(Failure::Protocol)
            );
        }
    }

    #[test]
    fn malformed_lifecycle_cancels_market_waits_without_interrupting_brti() {
        use metten_engine::sources::Stream;
        let memory = memory::Memory::default();
        let mut worker = SourceWorker::prepare(
            &super::super::tests::all_source_strategy(),
            &super::super::tests::source_markets(),
            memory.clone(),
        )
        .unwrap();
        let fence = Fence::new();
        worker
            .connection(
                Provider::Kalshi,
                worker.tokens()[0],
                metten_engine::sources::Connection::Open,
                0,
            )
            .unwrap();
        let progress = watch::channel(Progress::new()).0;
        let feed = |worker: &mut SourceWorker, bytes: &[u8]| {
            let mut item = Intake::new(
                Provider::Kalshi,
                &fence,
                memory.copy(bytes).unwrap(),
                Instant::now(),
            );
            item.source_tokens = Some(worker.tokens());
            item.receipt_ms = Some(0);
            item.progress = Some(progress.clone());
            worker.process(item, 0, 0, |_| Ok(()))
        };
        for (id, channel) in [
            (1, Channel::Book),
            (2, Channel::Trade),
            (3, Channel::Brti),
            (4, Channel::Lifecycle),
        ] {
            progress.send_modify(|p| {
                p.request(&Control::KalshiSubscribe {
                    id,
                    channel,
                    markets: if matches!(channel, Channel::Book | Channel::Trade) {
                        vec!["KXBTC15M-0".into()]
                    } else {
                        vec![]
                    },
                })
                .unwrap()
            });
        }
        for (sid, channel) in [
            (1, "orderbook_delta"),
            (2, "trade"),
            (3, "cfbenchmarks_value"),
            (4, "market_lifecycle_v2"),
        ] {
            feed(
                &mut worker,
                format!(r#"{{"type":"subscribed","msg":{{"sid":{sid},"channel":"{channel}"}}}}"#)
                    .as_bytes(),
            )
            .unwrap();
        }
        feed(&mut worker, br#"{"type":"cfbenchmarks_value_indexlist","sid":3,"seq":1,"msg":{"index_ids":["BRTI"]}}"#).unwrap();
        feed(&mut worker, br#"{"type":"cfbenchmarks_value","sid":3,"seq":2,"msg":{"index_id":"BRTI","data":"{\"id\":\"BRTI\",\"time\":1,\"value\":\"68000\"}"}}"#).unwrap();
        let before = worker.snapshot(0).unwrap();
        assert!(
            progress.borrow().deadline().is_some(),
            "books and trades still await data"
        );
        progress.send_modify(|p| {
            p.recover(Control::KalshiBookUpdate {
                id: 20,
                sid: 1,
                action: BookAction::Snapshot,
                markets: vec!["KXBTC15M-0".into()],
                token: None,
            })
        });
        assert_eq!(
            feed(
                &mut worker,
                br#"{"type":"market_lifecycle_v2","sid":4,"seq":1,"msg":{"market_ticker":42}}"#
            ),
            Err(Failure::Protocol)
        );
        feed(&mut worker, br#"{"type":"unsubscribed","sid":4,"seq":2}"#).unwrap();
        assert!(progress.borrow().deadline().is_none());
        assert!(progress.borrow().recovery.iter().all(Option::is_none));
        assert!(fence.current(fence.token()));
        assert!(before.stream_current(Stream::Brti));
        assert!(!before.stream_current(Stream::Book));
        let view = worker.snapshot(0).unwrap().wire_snapshot(0);
        assert!(
            view["bindings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|b| b["source"] == "kalshi" && b["field"] == "buyable_ask")
                .all(|b| b["reason"] == "invalid_value")
        );
    }

    #[test]
    fn known_brti_command_rejection_does_not_discard_contract_data() {
        let memory = memory::Memory::default();
        let strategy = super::super::tests::all_source_strategy();
        let markets = super::super::tests::source_markets();
        let mut worker = SourceWorker::prepare(&strategy, &markets, memory.clone()).unwrap();
        let fence = Fence::new();
        worker
            .connection(
                Provider::Kalshi,
                worker.tokens()[0],
                metten_engine::sources::Connection::Open,
                0,
            )
            .unwrap();
        let mut progress = Progress::new();
        progress
            .request(&Control::KalshiSubscribe {
                id: 3,
                channel: Channel::Brti,
                markets: vec![],
            })
            .unwrap();
        let progress = watch::channel(progress).0;
        let feed = |worker: &mut SourceWorker, bytes: &[u8]| {
            let mut item = Intake::new(
                Provider::Kalshi,
                &fence,
                memory.copy(bytes).unwrap(),
                Instant::now(),
            );
            item.source_tokens = Some(worker.tokens());
            item.receipt_ms = Some(0);
            item.progress = Some(progress.clone());
            worker.process(item, 0, 0, |_| Ok(()))
        };
        feed(
            &mut worker,
            br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#,
        )
        .unwrap();
        feed(&mut worker,br#"{"type":"orderbook_snapshot","sid":1,"seq":1,"msg":{"market_ticker":"KXBTC15M-0","yes_dollars_fp":[["0.36","1"]],"no_dollars_fp":[["0.64","1"]]}}"#).unwrap();
        let before = worker.snapshot(0).unwrap();
        feed(
            &mut worker,
            br#"{"type":"error","id":3,"msg":{"code":9,"msg":"synthetic refusal"}}"#,
        )
        .unwrap();
        assert!(fence.current(fence.token()));
        assert!(before.stream_current(metten_engine::sources::Stream::Book));
        assert!(!before.stream_current(metten_engine::sources::Stream::Brti));
        assert!(worker.state.snapshot(0).unwrap().bindings.iter().filter(|b|b.requirement.field=="buyable_ask" && matches!(&b.binding.scope,metten_engine::strategy::BindingScope::Contract {market_id,..} if market_id=="KXBTC15M-0")).all(|b|b.initialized));
        let view = worker.snapshot(0).unwrap().wire_snapshot(0);
        assert!(
            view["bindings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|b| b["source"] == "brti")
                .all(|b| b["reason"] == "access_refused")
        );
        let token = worker.tokens()[2];
        feed(&mut worker, br#"{"type":"error","id":3,"msg":{"code":9}}"#).unwrap();
        assert_eq!(worker.tokens()[2], token);
    }
}
