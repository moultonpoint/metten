use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::pkcs8::EncodePrivateKey;
use rsa::pkcs8::LineEnding;
use rsa::{pkcs1::EncodeRsaPrivateKey, signature::Verifier};
fn ed_pem() -> String {
    ed25519_dalek::SigningKey::from_bytes(&[7; 32])
        .to_pkcs8_pem(LineEnding::LF)
        .unwrap()
        .to_string()
}
#[test]
fn signatures_use_canonical_finite_requests() {
    let message = signing::presign(
        &Route::Markets {
            min_close_ts: 1,
            max_close_ts: 901,
            cursor: None,
        },
        123,
    )
    .unwrap();
    assert_eq!(message, "123GET/trade-api/v2/markets");
    let ed = signing::Key::parse(&ed_pem()).unwrap();
    let route = Route::Socket;
    let sig = STANDARD.decode(ed.sign(&route, 123).unwrap()).unwrap();
    ed25519_dalek::SigningKey::from_bytes(&[7; 32])
        .verifying_key()
        .verify_strict(
            b"123GET/trade-api/ws/v2",
            &ed25519_dalek::Signature::from_slice(&sig).unwrap(),
        )
        .unwrap();
    let rsa = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
    for pem in [
        rsa.to_pkcs8_pem(LineEnding::LF).unwrap(),
        rsa.to_pkcs1_pem(LineEnding::LF).unwrap(),
        format!(
            " \n{}\n",
            rsa.to_pkcs1_pem(LineEnding::LF).unwrap().as_str()
        )
        .into(),
    ] {
        let key = signing::Key::parse(&pem).unwrap();
        assert_eq!(key.kind(), "rsa");
        let sig = STANDARD.decode(key.sign(&route, 123).unwrap()).unwrap();
        rsa::pss::VerifyingKey::<sha2::Sha256>::new(rsa.to_public_key())
            .verify(
                b"123GET/trade-api/ws/v2",
                &rsa::pss::Signature::try_from(sig.as_slice()).unwrap(),
            )
            .unwrap();
    }
    for bad in [
        "garbage".into(),
        "x".repeat(16385),
        rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 1024)
            .unwrap()
            .to_pkcs8_pem(LineEnding::LF)
            .unwrap()
            .to_string(),
    ] {
        assert!(signing::Key::parse(&bad).is_err());
    }
}
#[test]
fn routes_cannot_escape_manifest() {
    assert_eq!(
        Route::Series.path().unwrap(),
        "/trade-api/v2/series/KXBTC15M"
    );
    for bad in ["../../orders", "KXBTC15M-a?x=y", "KXBTC15M-a/b", "OTHER"] {
        assert!(Route::Market(bad.into()).path().is_err());
    }
    for route in [
        Route::Markets {
            min_close_ts: 1,
            max_close_ts: 4000,
            cursor: None,
        },
        Route::Markets {
            min_close_ts: 1,
            max_close_ts: 2,
            cursor: Some("bad\ncursor".into()),
        },
    ] {
        assert!(route.path().is_err());
    }
    assert_eq!(Environment::Demo.account(), "kalshi:demo");
    assert_ne!(
        Environment::Demo.account(),
        Environment::Production.account()
    );
}
#[tokio::test(start_paused = true)]
async fn budgets_count_attempts_and_expire_before_spending() {
    use budget::{Budget, Traffic};
    let mut budget = Budget::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    for _ in 0..5 {
        budget.attempt(Traffic::Read, deadline).unwrap();
    }
    assert_eq!(
        budget.attempt(Traffic::Read, deadline),
        Err(Failure::Budget)
    );
    assert_eq!(budget.attempts, [5, 0, 0]);
    tokio::time::advance(Duration::from_secs(1)).await;
    budget.attempt(Traffic::Read, deadline).unwrap();
    tokio::time::advance(Duration::from_secs(9)).await;
    assert_eq!(
        budget.attempt(Traffic::Read, deadline),
        Err(Failure::Deadline)
    );
    assert_eq!(budget.attempts, [6, 0, 0]);
}
#[tokio::test]
async fn full_intake_revokes_without_waiting_and_obsolete_results_stay_revoked() {
    let (tx, mut rx) = intake();
    let fence = Fence::new();
    for _ in 0..RAW_SLOTS {
        admit(
            &tx,
            &fence,
            Intake::new(
                Provider::Kalshi,
                &fence,
                memory::Memory::default().copy(&[1]).unwrap(),
                Instant::now(),
            ),
        )
        .unwrap();
    }
    assert_eq!(
        admit(
            &tx,
            &fence,
            Intake::new(
                Provider::Kalshi,
                &fence,
                memory::Memory::default().copy(&[2]).unwrap(),
                Instant::now()
            )
        ),
        Err(Failure::Capacity)
    );
    assert!(!fence.current(rx.recv().await.unwrap().generation));
    let (stop, mut cancel) = watch::channel(false);
    stop.send_replace(true);
    assert_eq!(
        bounded(
            &mut cancel,
            Instant::now() + SHUTDOWN,
            std::future::pending::<Result<(), Failure>>()
        )
        .await,
        Err(Failure::Stopped)
    );
}
#[tokio::test]
async fn shutdown_retains_nonresponsive_worker_then_explicitly_recovers() {
    let mut owner = Coordinator::new();
    let (finish, wait) = std::sync::mpsc::channel();
    owner.worker = Some(std::thread::spawn(move || {
        wait.recv().unwrap();
    }));
    let old = owner.fence(Provider::Kalshi);
    let token = old.token();
    assert_eq!(
        owner
            .shutdown(Instant::now() + Duration::from_millis(10))
            .await,
        Err(Failure::StillStopping)
    );
    assert!(!old.current(token));
    assert!(owner.worker.is_some());
    assert_eq!(owner.recover(false), Err(Failure::StillStopping));
    finish.send(()).unwrap();
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.finished());
    owner.recover(false).unwrap();
    assert!(!old.current(token));
    assert!(owner.fence(Provider::Kalshi).current(1));
}
#[tokio::test]
async fn owned_async_exit_is_observed_and_deadline_cancels_work() {
    let mut owner = Coordinator::new();
    let mut cancel = owner.cancellation(Provider::Kalshi);
    owner
        .spawn(Provider::Kalshi, async move {
            let _ = cancel.changed().await;
        })
        .unwrap();
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.tasks.is_empty());
    let (_, mut stop) = watch::channel(false);
    assert_eq!(
        bounded(
            &mut stop,
            Instant::now() + Duration::from_millis(1),
            std::future::pending::<Result<(), Failure>>()
        )
        .await,
        Err(Failure::Stopped)
    );
}
#[tokio::test]
async fn controlled_http_rejects_redirect_and_large_body() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for response in [
        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/secret\r\nContent-Length: 0\r\n\r\n"
            .to_string(),
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
            MAX_BYTES + 1
        ),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let n = socket.read(&mut request).await.unwrap();
            assert!(
                std::str::from_utf8(&request[..n])
                    .unwrap()
                    .starts_with("GET / HTTP/1.1")
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        assert!(
            transport::test_body(&format!("http://{address}/"))
                .await
                .is_err()
        );
        task.await.unwrap();
    }
}
#[tokio::test]
async fn controlled_websocket_enforces_frame_limit() {
    use futures_util::{SinkExt, StreamExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        let _ = socket
            .send(tokio_tungstenite::tungstenite::Message::Text(
                "x".repeat(MAX_BYTES + 1).into(),
            ))
            .await;
    });
    let (mut socket, _) = tokio_tungstenite::connect_async_with_config(
        format!("ws://{address}"),
        Some(transport::config()),
        false,
    )
    .await
    .unwrap();
    assert!(socket.next().await.unwrap().is_err());
    drop(socket);
    peer.await.unwrap();
}
#[test]
fn credential_records_validate_before_mutation_and_readback_is_required() {
    use crate::provider_credential::{Credentials, Mutation, Status, Store};
    #[derive(Default)]
    struct Memory {
        items: [Option<Vec<u8>>; 2],
        fail: bool,
        ambiguous: bool,
    }
    impl Store for Memory {
        fn read(&mut self, e: Environment) -> Result<Option<Vec<u8>>, Failure> {
            Ok(self.items[usize::from(e == Environment::Production)].clone())
        }
        fn mutate(&mut self, e: Environment, _: Mutation, b: Option<&[u8]>) -> Result<(), Failure> {
            if self.fail {
                return Err(Failure::Unavailable);
            }
            if !self.ambiguous {
                self.items[usize::from(e == Environment::Production)] = b.map(Vec::from);
            }
            Ok(())
        }
    }
    let mut owner = Coordinator::new();
    owner.revoke();
    let mut credentials = Credentials::new(Memory::default());
    let env = Environment::Demo;
    let pem = ed_pem();
    assert_eq!(
        credentials.change(env, Mutation::Save, Some(("id", "bad")), &mut owner),
        Err(Failure::InvalidCredential)
    );
    assert_eq!(credentials.status(env), Ok(Status::Missing));
    credentials
        .change(env, Mutation::Save, Some(("id", &pem)), &mut owner)
        .unwrap();
    assert_eq!(
        credentials.change(env, Mutation::Save, Some(("other", &pem)), &mut owner),
        Err(Failure::AlreadySaved)
    );
    credentials
        .change(env, Mutation::Replace, Some(("other", &pem)), &mut owner)
        .unwrap();
    assert_eq!(
        credentials.status(env),
        Ok(Status::Saved {
            key_id: "other".into(),
            key_type: "ed25519"
        })
    );
    assert_eq!(
        credentials.status(Environment::Production),
        Ok(Status::Missing)
    );
    credentials
        .change(env, Mutation::Remove, None, &mut owner)
        .unwrap();
    assert_eq!(
        credentials.change(env, Mutation::Remove, None, &mut owner),
        Err(Failure::Missing)
    );
    let mut failed = Credentials::new(Memory {
        fail: true,
        ..Memory::default()
    });
    assert_eq!(
        failed.change(env, Mutation::Save, Some(("id", &pem)), &mut owner),
        Err(Failure::Unavailable)
    );
    let mut ambiguous = Credentials::new(Memory {
        ambiguous: true,
        ..Memory::default()
    });
    assert_eq!(
        ambiguous.change(env, Mutation::Save, Some(("id", &pem)), &mut owner),
        Err(Failure::Ambiguous)
    );
}

#[tokio::test(start_paused = true)]
async fn retry_is_bounded_and_stop_interrupts_backoff() {
    let mut retry = budget::Retry::new(7);
    for seconds in [5, 10, 20, 40, 60] {
        let delay = retry.delay(Failure::Transport).unwrap();
        if seconds == 60 {
            assert_eq!(delay, Duration::from_secs(60));
        } else {
            assert!(delay > Duration::from_secs(seconds));
        }
        assert!(delay <= Duration::from_secs(seconds) + Duration::from_millis(250));
    }
    assert!(retry.slow());
    // A credit at the failure instant would be ignored; step so this one starts the interval.
    tokio::time::advance(Duration::from_secs(1)).await;
    retry.usable(Instant::now());
    tokio::time::advance(Duration::from_secs(29)).await;
    retry.usable(Instant::now());
    assert!(retry.slow());
    tokio::time::advance(Duration::from_secs(1)).await;
    retry.usable(Instant::now());
    assert!(!retry.slow());
    assert!(budget::Retry::new(1).delay(Failure::Denied).is_none());
    let (stop, mut cancel) = watch::channel(false);
    let task = tokio::spawn(async move {
        bounded(
            &mut cancel,
            Instant::now() + Duration::from_secs(30),
            async {
                tokio::time::sleep(Duration::from_secs(20)).await;
                Ok(())
            },
        )
        .await
    });
    tokio::task::yield_now().await;
    stop.send_replace(true);
    assert_eq!(task.await.unwrap(), Err(Failure::Stopped));
}
#[test]
fn controls_have_exact_fixed_provider_scope() {
    let coinbase = Control::Coinbase { subscribe: true }
        .encode(Provider::Coinbase)
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&coinbase).unwrap(),
        serde_json::json!({"type":"subscribe","product_ids":["BTC-USD"],"channels":["matches","heartbeat"]})
    );
    assert!(
        Control::Coinbase { subscribe: true }
            .encode(Provider::Kalshi)
            .is_err()
    );
    assert!(
        Control::KalshiSubscribe {
            id: 1,
            channel: Channel::Book,
            markets: vec!["OTHER".into()]
        }
        .encode(Provider::Kalshi)
        .is_err()
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &Control::KalshiIndexes {
                id: 2,
                sid: 1,
                token: None
            }
            .encode(Provider::Kalshi)
            .unwrap()
        )
        .unwrap(),
        serde_json::json!({"id":2,"cmd":"update_subscription","params":{"sid":1,"action":"indexlist"}})
    );
    assert!(
        Control::KalshiSubscribe {
            id: 1,
            channel: Channel::Lifecycle,
            markets: vec!["KXBTC15M-A".into()]
        }
        .encode(Provider::Kalshi)
        .is_err()
    );
}
#[tokio::test]
async fn worker_exit_and_queue_are_owned_until_confirmed() {
    let mut owner = Coordinator::new();
    let (tx, rx) = intake();
    let (seen, received) = std::sync::mpsc::channel();
    owner
        .start_worker(rx, move |item| {
            seen.send(item.bytes.as_slice().to_vec()).unwrap();
        })
        .unwrap();
    let fence = owner.fence(Provider::Coinbase);
    admit(
        &tx,
        &fence,
        Intake::new(
            Provider::Coinbase,
            &fence,
            memory::Memory::default().copy(&[42]).unwrap(),
            Instant::now(),
        ),
    )
    .unwrap();
    // Avoid blocking the runtime while the native worker is scheduled.
    let output = tokio::task::spawn_blocking(move || received.recv_timeout(Duration::from_secs(1)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(output, vec![42]);
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.worker.is_none());
}

#[tokio::test]
async fn exact_outgoing_control_and_nonresponsive_peer_cancel_close_transport() {
    use futures_util::StreamExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        let text = socket.next().await.unwrap().unwrap().into_text().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).unwrap(),
            serde_json::json!({"type":"subscribe","product_ids":["BTC-USD"],"channels":["matches","heartbeat"]})
        );
        assert!(
            socket
                .next()
                .await
                .is_none_or(|m| m.is_err() || m.unwrap().is_close())
        );
    });
    let fence = Fence::new();
    let generation = fence.token();
    let mut connection =
        transport::local_connection(address, Provider::Coinbase, fence.clone()).await;
    let transport = transport::Transport::new(Environment::Demo).unwrap();
    let (_signal, mut stop) = watch::channel(false);
    transport
        .send(
            &mut connection,
            &Control::Coinbase { subscribe: true },
            &mut stop,
            Instant::now() + SHUTDOWN,
        )
        .await
        .unwrap();
    assert_eq!(
        transport
            .receive(
                &mut connection,
                &mut stop,
                Instant::now() + Duration::from_millis(10)
            )
            .await
            .map(|_| ()),
        Err(Failure::Deadline)
    );
    assert!(!fence.current(generation));
    assert_eq!(
        transport
            .send(
                &mut connection,
                &Control::Coinbase { subscribe: true },
                &mut stop,
                Instant::now() + SHUTDOWN
            )
            .await,
        Err(Failure::Stopped)
    );
    peer.await.unwrap();
}

#[test]
fn canceled_reservations_release_capacity_without_counting_an_attempt() {
    let budget = Arc::new(std::sync::Mutex::new(budget::Budget::new()));
    let deadline = Instant::now() + SHUTDOWN;
    let reserved =
        budget::Permit::reserve(budget.clone(), budget::Traffic::Read, deadline).unwrap();
    assert_eq!(budget.lock().unwrap().attempts, [0, 0, 0]);
    drop(reserved);
    for _ in 0..5 {
        let mut permit =
            budget::Permit::reserve(budget.clone(), budget::Traffic::Read, deadline).unwrap();
        permit.start().unwrap();
    }
    assert_eq!(budget.lock().unwrap().attempts, [5, 0, 0]);
    assert!(matches!(
        budget::Permit::reserve(budget, budget::Traffic::Read, deadline),
        Err(Failure::Budget)
    ));
}

#[tokio::test(start_paused = true)]
async fn expired_work_is_never_polled_and_late_success_is_not_current() {
    let (_signal, mut stop) = watch::channel(false);
    let called = std::sync::atomic::AtomicBool::new(false);
    assert_eq!(
        bounded(&mut stop, Instant::now(), async {
            called.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await,
        Err(Failure::Deadline)
    );
    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn adapter_memory_is_charged_before_growth_and_follows_retained_payloads() {
    let memory = memory::Memory::default();
    let held = memory.reserve(memory::ADAPTER_BYTES - 3).unwrap();
    let bytes = memory.copy(&[1, 2, 3]).unwrap();
    assert_eq!(memory.used(), memory::ADAPTER_BYTES);
    assert!(matches!(memory.copy(&[4]), Err(Failure::Capacity)));
    drop(bytes);
    assert_eq!(memory.used(), memory::ADAPTER_BYTES - 3);
    drop(held);
    assert_eq!(memory.used(), 0);
}
#[tokio::test]
async fn worker_rejects_old_buffering_and_accepts_a_new_explicit_generation() {
    let mut owner = Coordinator::new();
    let (tx, rx) = intake();
    let (seen, mut received) = tokio::sync::mpsc::channel(1);
    owner
        .start_worker(rx, move |item| {
            seen.try_send(item.bytes.as_slice().to_vec()).unwrap();
        })
        .unwrap();
    let old = owner.fence(Provider::Coinbase);
    admit(
        &tx,
        &old,
        Intake::new(
            Provider::Coinbase,
            &old,
            memory::Memory::default().copy(&[1]).unwrap(),
            Instant::now() - MAX_INTAKE_AGE - Duration::from_secs(1),
        ),
    )
    .unwrap();
    tokio::time::timeout(SHUTDOWN, async {
        while old.current(old.token()) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(received.try_recv().is_err());
    owner.recover_provider(Provider::Coinbase, false).unwrap();
    let fresh = owner.fence(Provider::Coinbase);
    admit(
        &tx,
        &fresh,
        Intake::new(
            Provider::Coinbase,
            &fresh,
            memory::Memory::default().copy(&[2]).unwrap(),
            Instant::now(),
        ),
    )
    .unwrap();
    assert_eq!(
        tokio::time::timeout(SHUTDOWN, received.recv())
            .await
            .unwrap(),
        Some(vec![2])
    );
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
}
#[tokio::test(start_paused = true)]
async fn revocation_interrupts_permit_wait_without_queue_capacity() {
    let fence = Fence::new();
    let task_fence = fence.clone();
    let work = tokio::spawn(async move {
        task_fence
            .run(
                Instant::now() + SHUTDOWN,
                std::future::pending::<Result<(), Failure>>(),
            )
            .await
    });
    tokio::task::yield_now().await;
    fence.revoke();
    assert_eq!(work.await.unwrap(), Err(Failure::Stopped));
}
#[tokio::test]
async fn partial_frame_timeout_drops_socket_and_never_returns_partial_data() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        socket.get_mut().write_all(&[0x81, 5, b'a']).await.unwrap();
        let mut b = [0; 1];
        assert_eq!(socket.get_mut().read(&mut b).await.unwrap(), 0);
    });
    let fence = Fence::new();
    let mut connection =
        transport::local_connection(address, Provider::Kraken, fence.clone()).await;
    let transport = transport::Transport::new(Environment::Demo).unwrap();
    let (_signal, mut stop) = watch::channel(false);
    assert!(matches!(
        transport
            .receive(
                &mut connection,
                &mut stop,
                Instant::now() + Duration::from_millis(30)
            )
            .await,
        Err(Failure::Deadline)
    ));
    assert!(!fence.current(fence.token()));
    peer.await.unwrap();
}

#[tokio::test]
async fn outgoing_get_ledger_counts_transient_retries_and_refuses_permanent_errors() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for transient in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            for attempt in 0..if transient { 2 } else { 1 } {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 4096];
                let n = socket.read(&mut request).await.unwrap();
                assert!(
                    std::str::from_utf8(&request[..n])
                        .unwrap()
                        .starts_with("GET /trade-api/v2/exchange/status HTTP/1.1\r\n")
                );
                let status = if transient && attempt == 0 {
                    "503 Service Unavailable"
                } else if transient {
                    "200 OK"
                } else {
                    "404 Not Found"
                };
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}").as_bytes()).await.unwrap();
            }
        });
        let transport = transport::Transport::controlled(address);
        let (_signal, mut stop) = watch::channel(false);
        let fence = Fence::new();
        let result = transport
            .get(
                &Route::ExchangeStatus,
                &mut stop,
                Instant::now() + Duration::from_secs(9),
                &fence,
            )
            .await;
        if transient {
            assert_eq!(result.unwrap().as_slice(), b"{}");
        } else {
            assert!(matches!(result, Err(Failure::Protocol)));
        }
        assert_eq!(
            transport.attempts(Provider::Kalshi).unwrap(),
            [if transient { 2 } else { 1 }, 0, 0]
        );
        peer.await.unwrap();
    }
}
#[tokio::test]
async fn denied_handshake_is_one_attempt_and_invalid_credential_is_zero() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        let n = socket.read(&mut request).await.unwrap();
        let request = std::str::from_utf8(&request[..n]).unwrap().to_lowercase();
        assert!(request.starts_with("get /trade-api/ws/v2 http/1.1"));
        assert!(request.contains("kalshi-access-key: controlled"));
        socket
            .write_all(
                b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
    });
    let transport = transport::Transport::controlled(address);
    let (_signal, mut stop) = watch::channel(false);
    let fence = Fence::new();
    assert!(matches!(
        transport
            .connect(
                Provider::Kalshi,
                None,
                &mut stop,
                Instant::now() + SHUTDOWN,
                &fence
            )
            .await,
        Err(Failure::Missing)
    ));
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap(), [0, 0, 0]);
    let key = signing::Key::parse(&ed_pem()).unwrap();
    assert!(matches!(
        transport
            .connect(
                Provider::Kalshi,
                Some(("controlled", &key)),
                &mut stop,
                Instant::now() + SHUTDOWN,
                &fence
            )
            .await,
        Err(Failure::Denied)
    ));
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap(), [0, 1, 0]);
    peer.await.unwrap();
}

#[tokio::test]
async fn partial_control_send_is_canceled_once_and_connection_is_discarded() {
    use tokio::io::AsyncReadExt;
    let (client, mut peer) = tokio::io::duplex(8);
    let mut socket = tokio_tungstenite::WebSocketStream::from_raw_socket(
        client,
        tokio_tungstenite::tungstenite::protocol::Role::Client,
        Some(transport::config()),
    )
    .await;
    let budget = Arc::new(std::sync::Mutex::new(budget::Budget::new()));
    let deadline = Instant::now() + SHUTDOWN;
    let mut permit =
        budget::Permit::reserve(budget.clone(), budget::Traffic::Control, deadline).unwrap();
    let (signal, mut stop) = watch::channel(false);
    let peer_task = tokio::spawn(async move {
        let mut first = [0; 1];
        peer.read_exact(&mut first).await.unwrap();
        signal.send_replace(true);
        let mut remainder = Vec::new();
        peer.read_to_end(&mut remainder).await.unwrap();
    });
    let command = Control::Coinbase { subscribe: true }
        .encode(Provider::Coinbase)
        .unwrap();
    assert_eq!(
        transport::send_control(&mut socket, command, &mut permit, &mut stop, deadline).await,
        Err(Failure::Stopped)
    );
    drop(socket);
    peer_task.await.unwrap();
    assert_eq!(budget.lock().unwrap().attempts, [0, 0, 1]);
}
#[tokio::test]
async fn body_stream_deadline_releases_owned_memory_and_connection() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nabc")
            .await
            .unwrap();
        let mut b = [0; 1];
        assert_eq!(socket.read(&mut b).await.unwrap(), 0);
    });
    let transport = transport::Transport::controlled(address);
    let (_signal, mut stop) = watch::channel(false);
    let fence = Fence::new();
    assert!(matches!(
        transport
            .get(
                &Route::ExchangeStatus,
                &mut stop,
                Instant::now() + Duration::from_millis(30),
                &fence
            )
            .await,
        Err(Failure::Deadline)
    ));
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap(), [1, 0, 0]);
    assert_eq!(transport.memory().used(), 0);
    peer.await.unwrap();
}

#[tokio::test]
async fn dropping_public_receive_after_a_partial_frame_revokes_and_closes() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        socket.get_mut().write_all(&[0x81, 5, b'a']).await.unwrap();
        let mut byte = [0; 1];
        assert_eq!(socket.get_mut().read(&mut byte).await.unwrap(), 0);
    });
    let fence = Fence::new();
    let mut connection =
        transport::local_connection(address, Provider::Kraken, fence.clone()).await;
    let transport = transport::Transport::new(Environment::Demo).unwrap();
    let (_signal, mut stop) = watch::channel(false);
    assert!(
        tokio::time::timeout(
            Duration::from_millis(30),
            transport.receive(&mut connection, &mut stop, Instant::now() + SHUTDOWN)
        )
        .await
        .is_err()
    );
    assert!(!fence.current(fence.token()));
    assert!(matches!(
        transport
            .receive(&mut connection, &mut stop, Instant::now() + SHUTDOWN)
            .await,
        Err(Failure::Stopped)
    ));
    peer.await.unwrap();
}
#[tokio::test]
async fn early_provider_return_and_panic_revoke_queued_evidence() {
    for panic in [false, true] {
        let mut owner = Coordinator::new();
        let fence = owner.fence(Provider::Kalshi);
        owner
            .spawn(Provider::Kalshi, async move {
                if panic {
                    panic!("controlled provider failure");
                }
            })
            .unwrap();
        tokio::task::yield_now().await;
        assert!(!fence.current(fence.token()));
        let (tx, _rx) = intake();
        assert_eq!(
            admit(
                &tx,
                &fence,
                Intake::new(
                    Provider::Kalshi,
                    &fence,
                    memory::Memory::default().copy(&[1]).unwrap(),
                    Instant::now()
                )
            ),
            Err(Failure::Stopped)
        );
        owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborted_nonresponsive_async_task_retains_ownership_until_actual_exit() {
    let mut owner = Coordinator::new();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    owner
        .spawn(Provider::Kalshi, async move {
            started.send(()).unwrap();
            wait.recv().unwrap();
        })
        .unwrap();
    ready.await.unwrap();
    assert_eq!(
        owner
            .stop_provider(Provider::Kalshi, Instant::now() + Duration::from_millis(10))
            .await,
        Err(Failure::StillStopping)
    );
    assert_eq!(
        owner.recover_provider(Provider::Kalshi, false),
        Err(Failure::StillStopping)
    );
    assert!(!owner.finished());
    release.send(()).unwrap();
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    owner.recover(false).unwrap();
}
#[tokio::test]
async fn worker_panic_revokes_all_current_generations_and_requires_joined_recovery() {
    let mut owner = Coordinator::new();
    let (tx, rx) = intake();
    let kalshi = owner.fence(Provider::Kalshi);
    let spot = owner.fence(Provider::Coinbase);
    owner
        .start_worker(rx, |_| panic!("controlled worker failure"))
        .unwrap();
    admit(
        &tx,
        &spot,
        Intake::new(
            Provider::Coinbase,
            &spot,
            memory::Memory::default().copy(&[1]).unwrap(),
            Instant::now(),
        ),
    )
    .unwrap();
    tokio::time::timeout(SHUTDOWN, async {
        while spot.current(spot.token()) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!kalshi.current(kalshi.token()));
    assert_eq!(
        owner.recover_provider(Provider::Kalshi, false),
        Err(Failure::StillStopping)
    );
    assert_eq!(
        owner.shutdown(Instant::now() + SHUTDOWN).await,
        Err(Failure::Unavailable)
    );
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    owner.recover(false).unwrap();
}

#[tokio::test(start_paused = true)]
async fn waiting_reservations_cannot_accumulate_a_later_burst() {
    let budget = Arc::new(std::sync::Mutex::new(budget::Budget::new()));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut held = Vec::new();
    for _ in 0..5 {
        held.push(
            budget::Permit::reserve(budget.clone(), budget::Traffic::Read, deadline).unwrap(),
        );
    }
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(matches!(
        budget::Permit::reserve(budget.clone(), budget::Traffic::Read, deadline),
        Err(Failure::Budget)
    ));
    for permit in &mut held {
        permit.start().unwrap();
    }
    assert!(matches!(
        budget::Permit::reserve(budget.clone(), budget::Traffic::Read, deadline),
        Err(Failure::Budget)
    ));
}

#[tokio::test]
async fn dropping_public_send_during_budget_wait_discards_the_connection() {
    use futures_util::StreamExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        assert!(socket.next().await.unwrap().unwrap().is_text());
        assert!(socket.next().await.is_none_or(|result| result.is_err()));
    });
    let fence = Fence::new();
    let mut connection =
        transport::local_connection(address, Provider::Coinbase, fence.clone()).await;
    let transport = transport::Transport::new(Environment::Demo).unwrap();
    let (_signal, mut stop) = watch::channel(false);
    transport
        .send(
            &mut connection,
            &Control::Coinbase { subscribe: true },
            &mut stop,
            Instant::now() + SHUTDOWN,
        )
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(10),
            transport.send(
                &mut connection,
                &Control::Coinbase { subscribe: false },
                &mut stop,
                Instant::now() + SHUTDOWN
            )
        )
        .await
        .is_err()
    );
    assert!(!fence.current(fence.token()));
    assert_eq!(transport.attempts(Provider::Coinbase).unwrap(), [0, 0, 1]);
    peer.await.unwrap();
}

#[tokio::test]
async fn protocol_pong_is_counted_without_spending_subscription_capacity() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        socket.send(Message::Ping(vec![1].into())).await.unwrap();
        assert!(socket.next().await.unwrap().unwrap().is_pong());
        socket.send(Message::Text("{}".into())).await.unwrap();
        assert!(socket.next().await.unwrap().unwrap().is_text());
    });
    let fence = Fence::new();
    let mut connection = transport::local_connection(address, Provider::Coinbase, fence).await;
    let transport = transport::Transport::new(Environment::Demo).unwrap();
    let (_signal, mut stop) = watch::channel(false);
    assert_eq!(
        transport
            .receive(&mut connection, &mut stop, Instant::now() + SHUTDOWN)
            .await
            .unwrap()
            .as_slice(),
        b"{}"
    );
    transport
        .send(
            &mut connection,
            &Control::Coinbase { subscribe: true },
            &mut stop,
            Instant::now() + Duration::from_millis(200),
        )
        .await
        .unwrap();
    assert_eq!(
        transport.counters(Provider::Coinbase).unwrap(),
        [0, 0, 1, 1, 1]
    );
    peer.await.unwrap();
}
#[tokio::test]
async fn metadata_client_never_has_more_than_two_requests_in_flight() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let transport = Arc::new(transport::Transport::controlled(address));
    let mut clients = Vec::new();
    for _ in 0..3 {
        let transport = transport.clone();
        clients.push(tokio::spawn(async move {
            let (_signal, mut stop) = watch::channel(false);
            transport
                .get(
                    &Route::ExchangeStatus,
                    &mut stop,
                    Instant::now() + SHUTDOWN,
                    &Fence::new(),
                )
                .await
        }));
    }
    let mut sockets = Vec::new();
    for _ in 0..2 {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        sockets.push(socket);
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(25), listener.accept())
            .await
            .is_err()
    );
    let mut first = sockets.remove(0);
    first
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
        .await
        .unwrap();
    drop(first);
    let (mut third, _) = listener.accept().await.unwrap();
    let mut request = [0; 4096];
    assert!(third.read(&mut request).await.unwrap() > 0);
    sockets.push(third);
    for mut socket in sockets {
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
            .await
            .unwrap();
    }
    for client in clients {
        assert_eq!(client.await.unwrap().unwrap().as_slice(), b"{}");
    }
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap(), [3, 0, 0]);
}
#[tokio::test]
async fn obsolete_admission_has_a_bounded_reason_counter() {
    let fence = Fence::new();
    let item = Intake::new(
        Provider::Kalshi,
        &fence,
        memory::Memory::default().copy(&[1]).unwrap(),
        Instant::now(),
    );
    let (tx, _rx) = intake();
    fence.revoke();
    assert_eq!(admit(&tx, &fence, item), Err(Failure::Stopped));
    assert_eq!(fence.rejected(), 1);
    assert_eq!(
        fence.run(Instant::now() + SHUTDOWN, async { Ok(()) }).await,
        Err(Failure::Stopped)
    );
    assert_eq!(fence.rejected(), 2);
}

#[tokio::test(start_paused = true)]
async fn price_accepted_before_a_failure_never_starts_the_next_interval() {
    let mut retry = budget::Retry::new(1);
    for _ in 0..5 {
        retry.delay(Failure::Transport);
    }
    tokio::time::advance(Duration::from_secs(1)).await;
    let accepted = Instant::now();
    tokio::time::advance(Duration::from_secs(1)).await;
    retry.delay(Failure::Transport);
    // The worker reaches the lock only after the failure.
    retry.usable(accepted);
    tokio::time::advance(Duration::from_secs(30)).await;
    retry.usable(Instant::now());
    assert!(
        retry.slow(),
        "the pre-failure price cannot start the interval"
    );
    tokio::time::advance(Duration::from_secs(30)).await;
    retry.usable(Instant::now());
    assert!(!retry.slow());
}

#[tokio::test(start_paused = true)]
async fn slow_retry_claims_a_minute_slot_even_when_the_attempt_succeeds() {
    let mut retry = budget::Retry::new(1);
    for _ in 0..5 {
        retry.delay(Failure::Transport).unwrap();
    }
    tokio::time::advance(Duration::from_secs(60)).await;
    assert!(retry.claim(Instant::now()).is_none());
    assert_eq!(retry.claim(Instant::now()), Some(Duration::from_secs(60)));
    retry.usable(Instant::now());
    tokio::time::advance(Duration::from_secs(30)).await;
    retry.usable(Instant::now());
    assert!(!retry.slow());
    assert!(retry.claim(Instant::now()).is_none());
    assert!(retry.claim(Instant::now()).is_none());
}

#[tokio::test]
async fn queued_handshake_rechecks_backoff_after_the_first_failure() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let transport = Arc::new(transport::Transport::controlled(address));
    let fence = Fence::new();
    let (_signal, stop) = watch::channel(false);
    let mut clients = Vec::new();
    for _ in 0..2 {
        let transport = transport.clone();
        let fence = fence.clone();
        let mut stop = stop.clone();
        clients.push(tokio::spawn(async move {
            transport
                .connect(
                    Provider::Coinbase,
                    None,
                    &mut stop,
                    Instant::now() + SHUTDOWN,
                    &fence,
                )
                .await
                .map(|_| ())
        }));
    }
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut request = [0; 4096];
    assert!(socket.read(&mut request).await.unwrap() > 0);
    tokio::task::yield_now().await;
    socket
        .write_all(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await
        .unwrap();
    drop(socket);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
    fence.revoke();
    for client in clients {
        assert!(matches!(client.await.unwrap(), Err(Failure::Stopped)));
    }
    assert_eq!(transport.attempts(Provider::Coinbase).unwrap(), [0, 1, 0]);
}

#[tokio::test]
async fn established_socket_loss_paces_reconnect_and_restarts_usable_window() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        for _ in 0..6 {
            let (socket, _) = listener.accept().await.unwrap();
            drop(tokio_tungstenite::accept_async(socket).await.unwrap());
        }
    });
    let transport = transport::Transport::controlled(address);
    let (_signal, mut stop) = watch::channel(false);
    let fence = Fence::new();
    let mut connection = transport
        .connect(
            Provider::Coinbase,
            None,
            &mut stop,
            Instant::now() + SHUTDOWN,
            &fence,
        )
        .await
        .unwrap();
    assert_eq!(
        transport
            .receive(&mut connection, &mut stop, Instant::now() + SHUTDOWN)
            .await
            .map(|_| ()),
        Err(Failure::Transport)
    );
    assert!(!fence.current(fence.token()));
    assert!(matches!(
        transport
            .connect(
                Provider::Coinbase,
                None,
                &mut stop,
                Instant::now() + Duration::from_millis(30),
                &Fence::new()
            )
            .await,
        Err(Failure::Deadline)
    ));
    assert_eq!(transport.attempts(Provider::Coinbase).unwrap(), [0, 1, 0]);
    // Direct fixture sockets exercise each failure without waiting through real backoff.
    for failure in 2..=6 {
        let fence = Fence::new();
        let mut connection = transport::local_connection(address, Provider::Coinbase, fence).await;
        if failure == 6 {
            transport
                .retry_usable(Provider::Coinbase, Instant::now())
                .unwrap();
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(20)).await;
            tokio::time::resume();
        }
        assert_eq!(
            transport
                .receive(&mut connection, &mut stop, Instant::now() + SHUTDOWN)
                .await
                .map(|_| ()),
            Err(Failure::Transport)
        );
        assert_eq!(
            transport.slow_retry(Provider::Coinbase).unwrap(),
            failure >= 5
        );
    }
    peer.await.unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(10)).await;
    transport
        .retry_usable(Provider::Coinbase, Instant::now())
        .unwrap();
    assert!(transport.slow_retry(Provider::Coinbase).unwrap());
    tokio::time::advance(Duration::from_secs(29)).await;
    transport
        .retry_usable(Provider::Coinbase, Instant::now())
        .unwrap();
    assert!(transport.slow_retry(Provider::Coinbase).unwrap());
    tokio::time::advance(Duration::from_secs(1)).await;
    transport
        .retry_usable(Provider::Coinbase, Instant::now())
        .unwrap();
    assert!(!transport.slow_retry(Provider::Coinbase).unwrap());
}

async fn step_clock(by: Duration) {
    tokio::time::pause();
    tokio::time::advance(by).await;
    tokio::time::resume();
}

/// A sixth production-configured connection, served after five real socket losses put its provider into
/// slow retry. Each frame passes from a local peer through `serve` into a display worker.
struct Served {
    transport: Arc<transport::Transport>,
    provider: Provider,
    worker: SourceWorker,
    start: Instant,
    frames: mpsc::UnboundedSender<String>,
    receipts: mpsc::Receiver<Intake>,
    signal: watch::Sender<bool>,
    service: tokio::task::JoinHandle<Result<(), Failure>>,
}
impl Served {
    async fn start(provider: Provider) -> Self {
        use futures_util::{SinkExt, StreamExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (frames, mut frames_rx) = mpsc::unbounded_channel::<String>();
        // The peer keeps accepting so a test can make the next attempt; the runtime ends it.
        drop(tokio::spawn(async move {
            for _ in 0..5 {
                let (socket, _) = listener.accept().await.unwrap();
                drop(tokio_tungstenite::accept_async(socket).await.unwrap());
            }
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            while let Some(frame) = frames_rx.recv().await {
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(frame.into()))
                    .await
                    .unwrap();
            }
            let _ = socket.next().await;
            let mut held = Vec::new();
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                held.push(tokio_tungstenite::accept_async(socket).await.unwrap());
            }
        }));
        let transport = Arc::new(transport::Transport::controlled(address));
        let key = signing::Key::parse(&ed_pem()).unwrap();
        let credential = (provider == Provider::Kalshi).then_some(("controlled", &key));
        let start = Instant::now();
        let mut worker = SourceWorker::prepare_display(transport.memory().clone()).unwrap();
        let (index, stream) = match provider {
            Provider::Kalshi => (1, metten_engine::sources::Stream::Book),
            Provider::Coinbase => (2, metten_engine::sources::Stream::Coinbase),
            Provider::Kraken => (3, metten_engine::sources::Stream::Kraken),
        };
        let mut enabled = [false; 4];
        enabled[index] = true;
        worker.membership(enabled, 0).unwrap();
        worker
            .connection(
                provider,
                worker.state.token(stream),
                metten_engine::sources::Connection::Open,
                0,
            )
            .unwrap();
        let tokens = worker.tokens();
        let (signal, stop) = watch::channel(false);
        let connect = || {
            let transport = transport.clone();
            let mut stop = stop.clone();
            async move {
                transport
                    .connect(
                        provider,
                        credential,
                        &mut stop,
                        Instant::now() + SHUTDOWN,
                        &Fence::new(),
                    )
                    .await
                    .unwrap()
            }
        };
        for _ in 0..5 {
            step_clock(Duration::from_secs(61)).await;
            let connection = connect().await;
            let (_controls, mut controls_rx) = mpsc::channel(1);
            let (intake, _receipts) = intake();
            let progress = Some(watch::channel(transport::Progress::new()).0);
            assert_eq!(
                transport
                    .serve(
                        connection,
                        &mut controls_rx,
                        intake,
                        tokens,
                        stop.clone(),
                        progress
                    )
                    .await,
                Err(Failure::Transport)
            );
        }
        assert!(transport.slow_retry(provider).unwrap());
        step_clock(Duration::from_secs(61)).await;
        let connection = connect().await;
        let (controls, mut controls_rx) = mpsc::channel(1);
        let (intake, receipts) = intake();
        let served = transport.clone();
        let service = tokio::spawn(async move {
            let _controls = controls;
            served
                .serve(
                    connection,
                    &mut controls_rx,
                    intake,
                    tokens,
                    stop,
                    Some(watch::channel(transport::Progress::new()).0),
                )
                .await
        });
        Self {
            transport,
            provider,
            worker,
            start,
            frames,
            receipts,
            signal,
            service,
        }
    }
    /// The peer sends `frame`; the worker processes it at the current controlled time.
    async fn frame(&mut self, frame: String) {
        self.frames.send(frame).unwrap();
        let item = tokio::time::timeout(SHUTDOWN, self.receipts.recv())
            .await
            .unwrap()
            .unwrap();
        let at = u64::try_from(self.start.elapsed().as_millis()).unwrap();
        self.worker.process(item, at, 0, |_| Ok(())).unwrap();
    }
    /// Frames one second of controlled clock apart, with no step after the last.
    async fn frames(&mut self, frames: impl IntoIterator<Item = String>) {
        for (i, frame) in frames.into_iter().enumerate() {
            if i > 0 {
                step_clock(Duration::from_secs(1)).await;
            }
            self.frame(frame).await;
        }
    }
    /// Stops at once; returns whether slow retry is still on.
    async fn stop(self) -> (Arc<transport::Transport>, bool) {
        self.signal.send_replace(true);
        assert_eq!(
            tokio::time::timeout(SHUTDOWN, self.service)
                .await
                .unwrap()
                .unwrap(),
            Err(Failure::Stopped)
        );
        let slow = self.transport.slow_retry(self.provider).unwrap();
        (self.transport, slow)
    }
}

fn coinbase_match(trade_id: u64) -> String {
    let kind = if trade_id == 10 {
        "last_match"
    } else {
        "match"
    };
    format!(
        r#"{{"type":"{kind}","product_id":"BTC-USD","trade_id":{trade_id},"price":"68000.12","time":"2026-11-16T00:00:00Z"}}"#
    )
}
fn coinbase_heartbeat(last_trade_id: u64) -> String {
    format!(r#"{{"type":"heartbeat","product_id":"BTC-USD","last_trade_id":{last_trade_id}}}"#)
}
fn brti_index_list(seq: u64, present: bool) -> String {
    let ids = if present { r#"["BRTI"]"# } else { "[]" };
    format!(
        r#"{{"type":"cfbenchmarks_value_indexlist","sid":3,"seq":{seq},"msg":{{"index_ids":{ids}}}}}"#
    )
}

#[tokio::test]
async fn price_thirty_seconds_after_the_first_ends_slow_retry_even_when_stopped_at_once() {
    let mut served = Served::start(Provider::Coinbase).await;
    // 31 prices one second apart span 30 s; the connection stops right after the last is accepted.
    served.frames((10..41).map(coinbase_match)).await;
    let (transport, slow) = served.stop().await;
    assert!(!slow);
    let (_signal, mut stop) = watch::channel(false);
    assert!(
        transport
            .connect(
                Provider::Coinbase,
                None,
                &mut stop,
                Instant::now() + Duration::from_millis(500),
                &Fence::new()
            )
            .await
            .is_ok(),
        "the next attempt must not wait out the one-minute slow pace"
    );
}

#[tokio::test]
async fn handshake_or_twenty_nine_seconds_of_prices_keep_slow_retry() {
    let served = Served::start(Provider::Coinbase).await;
    for _ in 0..31 {
        step_clock(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
    }
    assert_eq!(
        tokio::time::timeout(SHUTDOWN, served.service)
            .await
            .unwrap()
            .unwrap(),
        Err(Failure::Deadline)
    );
    assert!(served.transport.slow_retry(Provider::Coinbase).unwrap());
    let mut served = Served::start(Provider::Coinbase).await;
    served.frames((10..40).map(coinbase_match)).await;
    assert!(served.stop().await.1);
}

#[tokio::test]
async fn late_reader_cannot_stretch_twenty_nine_seconds_of_prices() {
    let mut served = Served::start(Provider::Coinbase).await;
    served.frames((10..40).map(coinbase_match)).await;
    step_clock(Duration::from_secs(2)).await;
    served.frame(coinbase_heartbeat(39)).await;
    assert!(served.stop().await.1);
}

#[tokio::test]
async fn shared_failure_between_acceptance_and_credit_restarts_the_thirty_seconds() {
    let mut served = Served::start(Provider::Coinbase).await;
    served.frames.send(coinbase_match(10)).unwrap();
    let item = tokio::time::timeout(SHUTDOWN, served.receipts.recv())
        .await
        .unwrap()
        .unwrap();
    let retry = item.retry.clone().unwrap();
    let at = u64::try_from(served.start.elapsed().as_millis()).unwrap();
    let runtime = tokio::runtime::Handle::current();
    let worker = &mut served.worker;
    tokio::time::pause();
    std::thread::scope(|scope| {
        let mut held = retry.lock().unwrap();
        let processed = scope.spawn(move || {
            let _clock = runtime.enter();
            worker.process(item, at, 0, |_| Ok(()))
        });
        // The clock is paused, so the price is accepted no later than this failure, which its credit follows.
        held.delay(Failure::Transport);
        drop(held);
        processed.join().unwrap().unwrap();
    });
    tokio::time::resume();
    for _ in 0..31 {
        step_clock(Duration::from_secs(1)).await;
        served.frame(coinbase_heartbeat(10)).await;
    }
    served.frame(coinbase_match(11)).await;
    assert!(served.stop().await.1);
}

#[tokio::test]
async fn frames_without_an_accepted_price_value_keep_slow_retry() {
    let mut served = Served::start(Provider::Coinbase).await;
    served.frames(
        std::iter::once(r#"{"type":"subscriptions","channels":[{"name":"matches","product_ids":["BTC-USD"]},{"name":"heartbeat","product_ids":["BTC-USD"]}]}"#.to_owned())
            .chain((0..34).map(|_| coinbase_heartbeat(1))),
    )
    .await;
    assert!(served.stop().await.1);
    // One accepted BRTI value, then 31 s of index-list heartbeats ending with an absent reading,
    // which would complete the interval if it were credited.
    let mut served = Served::start(Provider::Kalshi).await;
    served
        .frames([
            r#"{"type":"subscribed","msg":{"sid":3,"channel":"cfbenchmarks_value"}}"#.to_owned(),
            brti_index_list(1, true),
            r#"{"type":"cfbenchmarks_value","sid":3,"seq":2,"msg":{"index_id":"BRTI","data":"{\"id\":\"BRTI\",\"time\":1,\"value\":\"68000.12\"}"}}"#.to_owned(),
        ])
        .await;
    for seq in 3..34 {
        step_clock(Duration::from_secs(1)).await;
        served.frame(brti_index_list(seq, seq != 33)).await;
    }
    assert!(served.stop().await.1);
}

#[tokio::test(start_paused = true)]
async fn only_frames_with_an_accepted_price_value_are_credited() {
    use metten_engine::sources::Connection;
    let mut worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        memory::Memory::default(),
    )
    .unwrap();
    worker
        .connection(Provider::Kalshi, worker.tokens()[0], Connection::Open, 0)
        .unwrap();
    let fence = Fence::new();
    let retry = Arc::new(std::sync::Mutex::new(budget::Retry::new(1)));
    for _ in 0..5 {
        retry.lock().unwrap().delay(Failure::Transport);
    }
    let subscribed = br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#;
    let priced = book_frame(1, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#);
    // An empty book still sends value-less updates that expire the initialized sides.
    let empty = book_frame(2, "KXBTC15M-0", "[]", "[]");
    let repriced = book_frame(3, "KXBTC15M-0", r#"[["0.40","1"]]"#, r#"[["0.60","1"]]"#);
    // Frames 30 s apart: crediting the acknowledgment or the empty book would clear slow retry one frame early.
    for (seq, bytes, slow) in [
        (0, subscribed.as_slice(), true),
        (1, priced.as_bytes(), true),
        (2, empty.as_bytes(), true),
        (3, repriced.as_bytes(), false),
    ] {
        if seq > 0 {
            tokio::time::advance(Duration::from_secs(30)).await;
        }
        let mut item = Intake::new(
            Provider::Kalshi,
            &fence,
            worker.memory.copy(bytes).unwrap(),
            Instant::now(),
        );
        item.source_tokens = Some(worker.tokens());
        item.receipt_ms = Some(seq * 30_000);
        item.retry = Some(retry.clone());
        worker.process(item, seq * 30_000, 0, |_| Ok(())).unwrap();
        assert_eq!(retry.lock().unwrap().slow(), slow, "frame {seq}");
    }
}

#[tokio::test]
async fn session_parent_stop_cancels_wait_and_old_sessions_never_revive() {
    let parent = Fence::new();
    let old = parent.session().unwrap();
    let old_token = old.token();
    old.revoke();
    assert!(!old.current(old_token));
    let session = parent.session().unwrap();
    let fence = session.clone();
    let work = tokio::spawn(async move {
        fence
            .run(
                Instant::now() + SHUTDOWN,
                std::future::pending::<Result<(), Failure>>(),
            )
            .await
    });
    tokio::task::yield_now().await;
    parent.revoke();
    assert_eq!(work.await.unwrap(), Err(Failure::Stopped));
    assert!(!session.current(session.token()));
    assert!(parent.session().is_err());
}

#[tokio::test]
async fn owned_service_preserves_partial_receive_during_control_and_stops() {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        socket.get_mut().write_all(&[0x81, 2, b'{']).await.unwrap();
        assert!(socket.next().await.unwrap().unwrap().is_text());
        socket.get_mut().write_all(b"}").await.unwrap();
        assert!(
            socket
                .next()
                .await
                .is_none_or(|r| r.is_err() || r.unwrap().is_close())
        );
    });
    let parent = Fence::new();
    let fence = parent.session().unwrap();
    let connection = transport::local_connection(address, Provider::Coinbase, fence.clone()).await;
    let transport = Arc::new(transport::Transport::new(Environment::Demo).unwrap());
    let (tx, mut rx) = mpsc::channel(1);
    let (intake, mut received) = intake();
    let (signal, stop) = watch::channel(false);
    let strategy = metten_engine::strategy::ValidatedStrategy::parse(include_str!(
        "../../../examples/learning.json"
    ))
    .unwrap();
    let source = metten_engine::sources::SourceState::new(&strategy, &[], &[], &[]).unwrap();
    let tokens = SOURCE_STREAMS.map(|s| source.token(s));
    let running = transport.clone();
    let task = tokio::spawn(async move {
        running
            .serve(connection, &mut rx, intake, tokens, stop, None)
            .await
    });
    tx.send(Control::Coinbase { subscribe: true })
        .await
        .unwrap();
    let item = tokio::time::timeout(SHUTDOWN, received.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.bytes.as_slice(), b"{}");
    assert!(fence.current(fence.token()));
    signal.send_replace(true);
    assert_eq!(task.await.unwrap(), Err(Failure::Stopped));
    peer.await.unwrap();
    assert!(!fence.current(fence.token()));
}

#[test]
fn real_book_decoder_drives_learning_strategy_without_changing_its_rules() {
    use metten_engine::{sources::*, strategy::*};
    fn complete(e: &mut Evaluator, effect: SourceEffect) -> Decision {
        e.begin(effect.batch).unwrap();
        loop {
            let range = PriceRange {
                minimum: Decimal::parse("1").unwrap(),
                maximum: Decimal::parse("99").unwrap(),
            };
            let objective = ObjectiveView {
                market_id: "KXBTC15M-M".into(),
                requested_quantity: 1,
                confirmed_quantity: 0,
                locked_side: None,
                attempt_state: AttemptState::None,
                safeguards: Safeguards {
                    allowed: true,
                    yes: range.clone(),
                    no: range,
                },
            };
            match e.process_slice(SliceControl::Continue, Some(objective)) {
                SliceResult::Pending { .. } => (),
                SliceResult::Complete(d) => return d,
                other => panic!("{other:?}"),
            }
        }
    }
    let strategy =
        ValidatedStrategy::parse(include_str!("../../../examples/learning.json")).unwrap();
    let market = MarketDescriptor {
        series_id: "KXBTC15M".into(),
        market_id: "KXBTC15M-M".into(),
        contract_id: "contract-m".into(),
        open_ms: 1,
        close_ms: 900001,
    };
    let mut bindings = Vec::new();
    for r in strategy.source_requirements() {
        let sides = match r.side {
            Some(SourceSide::Selected) => vec![Side::Yes, Side::No],
            Some(SourceSide::Fixed(s)) => vec![s],
            None => panic!("ask fixture"),
        };
        for side in sides {
            bindings.push(BindingDescriptor {
                binding_id: bindings.len() as u32 + 1,
                input_id: r.input_id.clone(),
                scope: BindingScope::Contract {
                    market_id: market.market_id.clone(),
                    contract_id: market.contract_id.clone(),
                    side,
                },
            })
        }
    }
    let caps: Vec<_> = bindings
        .iter()
        .map(|b| {
            (
                b.binding_id,
                Capabilities {
                    current: None,
                    observations: None,
                    history: None,
                },
            )
        })
        .collect();
    let source =
        SourceState::new(&strategy, std::slice::from_ref(&market), &bindings, &caps).unwrap();
    let mut e = Evaluator::new(
        strategy,
        RunContext {
            strategy_version: "learning".into(),
            run_id: "controlled".into(),
            go_time: "2026-11-16T00:00:00Z".into(),
            series_id: "KXBTC15M".into(),
            markets: vec![market.clone()],
            bindings,
        },
    )
    .unwrap();
    let memory = memory::Memory::default();
    let mut worker = SourceWorker::new(source, memory.clone()).unwrap();
    worker.lease = Some(1_000_000);
    let connection = worker
        .connection(
            super::Provider::Kalshi,
            worker.state.token(Stream::Book),
            Connection::Open,
            0,
        )
        .unwrap();
    complete(&mut e, connection);
    let fence = Fence::new();
    let mut ack = Intake::new(
        super::Provider::Kalshi,
        &fence,
        memory
            .copy(br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#)
            .unwrap(),
        Instant::now(),
    );
    ack.source_tokens = Some(worker.tokens());
    ack.receipt_ms = Some(0);
    worker
        .process(ack, 0, e.snapshot().evaluated_through_ms, |effect| {
            complete(&mut e, effect);
            Ok(())
        })
        .unwrap();
    let feed = |worker: &mut SourceWorker, e: &mut Evaluator, seq: u64, at: u64, yes: &str| {
        let no = if yes == "0.75" { "0.25" } else { "0.64" };
        let bytes = format!(
            r#"{{"type":"orderbook_snapshot","sid":1,"seq":{seq},"msg":{{"market_ticker":"KXBTC15M-M","yes_dollars_fp":[["{yes}","2"]],"no_dollars_fp":[["{no}","3"]]}}}}"#
        );
        let mut item = Intake::new(
            super::Provider::Kalshi,
            &fence,
            memory.copy(bytes.as_bytes()).unwrap(),
            Instant::now(),
        );
        item.source_tokens = Some(worker.tokens());
        item.receipt_ms = Some(at);
        let mut result = None;
        worker
            .process(item, at, e.snapshot().evaluated_through_ms, |effect| {
                result = Some(complete(e, effect));
                Ok(())
            })
            .unwrap();
        result.unwrap()
    };
    feed(&mut worker, &mut e, 1, 0, "0.36");
    let effect = worker
        .state
        .apply(source_envelope(
            0,
            SourceEvent::Metadata {
                token: worker.state.token(Stream::Metadata),
                market_id: market.market_id.clone(),
                verified: true,
            },
        ))
        .unwrap();
    complete(&mut e, effect);
    let effect = worker
        .state
        .apply(source_envelope(
            1,
            SourceEvent::Boundary {
                market_id: market.market_id.clone(),
            },
        ))
        .unwrap();
    let first = complete(&mut e, effect);
    assert!(first.checkpoints.contains_key("early_side_checkpoint"));
    let later = feed(&mut worker, &mut e, 2, 60001, "0.75");
    assert_eq!(
        later.checkpoints["early_side_checkpoint"].side,
        Some(Side::Yes)
    );
    assert!(
        later
            .leaves
            .iter()
            .any(|l| l.option_id == "opposite_side_first" && l.eligible)
    );
    assert_eq!(later.candidate.as_ref().unwrap().side, Side::No);
    let old = worker.tokens();
    fence.revoke();
    let mut stale = Intake::new(
        super::Provider::Kalshi,
        &fence,
        memory.copy(b"{}").unwrap(),
        Instant::now(),
    );
    stale.source_tokens = Some(old);
    assert_eq!(
        worker.process(stale, 60002, e.snapshot().evaluated_through_ms, |_| Ok(())),
        Err(Failure::Stopped)
    );
}

#[tokio::test]
async fn fixed_provider_task_and_owned_source_worker_publish_only_verified_values() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        let subscription = socket.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(subscription.contains("BTC-USD"));
        socket.send(Message::Text(r#"{"type":"last_match","product_id":"BTC-USD","trade_id":10,"price":"68000.12","time":"2026-11-16T00:00:00Z"}"#.into())).await.unwrap();
        assert!(
            socket
                .next()
                .await
                .is_none_or(|m| m.is_err() || m.unwrap().is_close())
        );
    });
    let strategy=metten_engine::strategy::ValidatedStrategy::parse(r#"{"format":2,"name":"Source task","inputs":[{"id":"spot","source":"coinbase","instrument":"btc_usd","field":"last_trade","unit":"usd_per_btc"}],"purchase_options":[{"id":"show","side":{"type":"fixed","side":"yes"},"when":{"type":"compare","left":{"reference":"spot"},"comparison":"greater_than","right":{"value":"1","unit":"usd_per_btc"}}}]}"#).unwrap();
    let transport = Arc::new(transport::Transport::controlled(address));
    let worker = SourceWorker::prepare(&strategy, &[], transport.memory().clone()).unwrap();
    let mut owner = Coordinator::new();
    let mut handle = owner.start_sources(worker).unwrap();
    owner
        .start_provider(Provider::Coinbase, transport.clone(), &mut handle, None)
        .unwrap();
    tokio::time::timeout(SHUTDOWN, async {
        loop {
            let received = handle
                .latest
                .borrow()
                .as_ref()
                .and_then(|v| v.snapshot().ok())
                .is_some_and(|v| {
                    v.bindings.iter().any(|b| {
                        b.value.is_some_and(|v| v.millionths() == 68000120000) && !b.usable
                    })
                });
            if received {
                break;
            }
            handle.latest.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(transport.attempts(Provider::Coinbase).unwrap(), [0, 1, 1]);
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.finished());
    peer.await.unwrap();
}

pub(super) fn all_source_strategy() -> metten_engine::strategy::ValidatedStrategy {
    use serde_json::json;
    let mut learning: serde_json::Value =
        serde_json::from_str(include_str!("../../../examples/learning.json")).unwrap();
    let inputs = learning["inputs"].as_array_mut().unwrap();
    for (id, source, field) in [
        ("trade", "kalshi", "last_trade"),
        ("index", "brti", "index_value"),
        ("coin", "coinbase", "last_trade"),
        ("krak", "kraken", "last_trade"),
    ] {
        let mut input = json!({"id":id,"source":source,"instrument":"btc_usd","field":field,"unit":"usd_per_btc"});
        if source == "kalshi" {
            input["instrument"] = json!("current_contract");
            input["unit"] = json!("contract_cents");
            input["side"] = json!("selected_side");
        }
        inputs.push(input);
    }
    metten_engine::strategy::ValidatedStrategy::parse(&learning.to_string()).unwrap()
}
pub(super) fn source_markets() -> Vec<metten_engine::strategy::MarketDescriptor> {
    [0, 900000]
        .into_iter()
        .enumerate()
        .map(|(i, open)| metten_engine::strategy::MarketDescriptor {
            series_id: "KXBTC15M".into(),
            market_id: format!("KXBTC15M-{i}"),
            contract_id: format!("contract-{i}"),
            open_ms: open,
            close_ms: open + 900000,
        })
        .collect()
}
fn feed_source(
    worker: &mut SourceWorker,
    fence: &Fence,
    provider: Provider,
    bytes: &[u8],
    at: u64,
    tokens: Option<[metten_engine::sources::StreamToken; 6]>,
) -> Result<(), Failure> {
    let mut item = Intake::new(
        provider,
        fence,
        worker.memory.copy(bytes).unwrap(),
        Instant::now(),
    );
    item.source_tokens = Some(tokens.unwrap_or_else(|| worker.tokens()));
    item.receipt_ms = Some(at);
    worker.process(item, at, 0, |_| Ok(()))
}
fn book_frame(seq: u64, ticker: &str, yes: &str, no: &str) -> String {
    format!(
        r#"{{"type":"orderbook_snapshot","sid":1,"seq":{seq},"msg":{{"market_ticker":"{ticker}","yes_dollars_fp":{yes},"no_dollars_fp":{no}}}}}"#
    )
}
#[test]
fn empty_book_expires_old_side_without_poisoning_following_reconstruction() {
    use metten_engine::sources::*;
    let mut worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        memory::Memory::default(),
    )
    .unwrap();
    worker
        .connection(
            super::Provider::Kalshi,
            worker.tokens()[0],
            Connection::Open,
            0,
        )
        .unwrap();
    let fence = Fence::new();
    feed_source(
        &mut worker,
        &fence,
        super::Provider::Kalshi,
        br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#,
        0,
        None,
    )
    .unwrap();
    for (seq, yes, no) in [
        (1, r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#),
        (2, "[]", "[]"),
        (3, r#"[["0.40","1"]]"#, r#"[["0.60","1"]]"#),
    ] {
        let tokens = worker.tokens();
        let old = worker.snapshot(seq).unwrap();
        feed_source(
            &mut worker,
            &fence,
            super::Provider::Kalshi,
            book_frame(seq, "KXBTC15M-0", yes, no).as_bytes(),
            seq,
            None,
        )
        .unwrap();
        let view = worker.state.snapshot(seq).unwrap();
        let asks:Vec<_>=view.bindings.iter().filter(|b|b.requirement.field=="buyable_ask" && matches!(&b.binding.scope,metten_engine::strategy::BindingScope::Contract {market_id,..} if market_id=="KXBTC15M-0")).collect();
        assert!(asks.iter().all(|b| b.initialized == (seq != 2)));
        assert_eq!(
            worker.tokens(),
            tokens,
            "empty liquidity must not strand the subscription token"
        );
        if seq == 2 {
            assert!(!old.stream_current(Stream::Book));
            assert!(asks.iter().all(|b| b.value.is_none() && !b.usable));
        }
    }
}
#[test]
fn shared_book_gap_revokes_both_and_stale_completion_cannot_restore_either_or_brti() {
    use metten_engine::sources::{Connection, Stream};
    let mut worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        memory::Memory::default(),
    )
    .unwrap();
    worker
        .connection(Provider::Kalshi, worker.tokens()[0], Connection::Open, 0)
        .unwrap();
    let fence = Fence::new();
    for (sid, channel) in [(1, "orderbook_delta"), (2, "cfbenchmarks_value")] {
        feed_source(
            &mut worker,
            &fence,
            Provider::Kalshi,
            format!(r#"{{"type":"subscribed","msg":{{"sid":{sid},"channel":"{channel}"}}}}"#)
                .as_bytes(),
            0,
            None,
        )
        .unwrap();
    }
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        br#"{"type":"cfbenchmarks_value_indexlist","sid":2,"seq":1,"msg":{"index_ids":["BRTI"]}}"#,
        0,
        None,
    )
    .unwrap();
    feed_source(&mut worker,&fence,Provider::Kalshi,br#"{"type":"cfbenchmarks_value","sid":2,"seq":2,"msg":{"index_id":"BRTI","data":"{\"id\":\"BRTI\",\"time\":1,\"value\":\"68000.12\"}"}}"#,0,None).unwrap();
    for (seq, ticker) in [(1, "KXBTC15M-0"), (2, "KXBTC15M-1")] {
        feed_source(
            &mut worker,
            &fence,
            Provider::Kalshi,
            book_frame(seq, ticker, r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#).as_bytes(),
            0,
            None,
        )
        .unwrap();
    }
    let before = worker.snapshot(0).unwrap();
    let old = worker.tokens();
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        book_frame(4, "KXBTC15M-0", "[]", "[]").as_bytes(),
        1,
        None,
    )
    .unwrap();
    assert!(!before.stream_current(Stream::Book));
    assert!(before.stream_current(Stream::Brti));
    assert!(fence.current(fence.token()));
    assert_eq!(
        worker.tokens()[Stream::Brti as usize],
        old[Stream::Brti as usize]
    );
    assert_eq!(
        feed_source(
            &mut worker,
            &fence,
            Provider::Kalshi,
            book_frame(5, "KXBTC15M-0", "[]", "[]").as_bytes(),
            1,
            Some(old)
        ),
        Err(Failure::Stopped)
    );
    for (seq, ticker) in [(6, "KXBTC15M-0"), (7, "KXBTC15M-1")] {
        feed_source(
            &mut worker,
            &fence,
            Provider::Kalshi,
            book_frame(seq, ticker, r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#).as_bytes(),
            2,
            None,
        )
        .unwrap();
        let initialized: Vec<_> = worker
            .state
            .snapshot(2)
            .unwrap()
            .bindings
            .into_iter()
            .filter(|b| b.requirement.field == "buyable_ask")
            .collect();
        assert_eq!(
            initialized.iter().filter(|b| b.initialized).count(),
            if seq == 6 { 2 } else { 4 }
        );
        assert!(initialized.iter().all(|b| !b.usable));
    }
    let before_bad = worker.snapshot(2).unwrap();
    assert_eq!(feed_source(&mut worker,&fence,Provider::Kalshi,br#"{"type":"cfbenchmarks_value","sid":2,"seq":3,"msg":{"index_id":"BRTI","data":"{\"id\":\"ETHUSD_RTI\",\"time\":2,\"value\":\"1\"}"}}"#,3,None),Err(Failure::Protocol));
    assert!(fence.current(fence.token()));
    assert!(before_bad.stream_current(Stream::Book));
    assert!(!before_bad.stream_current(Stream::Brti));
    // An observed unsubscribe completion belongs to the same subscription sequence.
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        br#"{"type":"unsubscribed","id":1,"sid":2,"seq":4}"#,
        3,
        None,
    )
    .unwrap();
    let old = worker.snapshot(3).unwrap();
    assert!(
        old.wire_snapshot(3)["bindings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["source"] == "brti")
            .all(|b| b["reason"] == "invalid_value")
    );
    assert_eq!(
        feed_source(&mut worker, &fence, Provider::Kalshi, b"{", 4, None),
        Err(Failure::Protocol)
    );
    assert!(!old.stream_current(Stream::Brti));
    assert!(!fence.current(fence.token()));
}

#[tokio::test]
async fn discovery_uses_six_then_four_fixed_gets_and_worker_rejects_obsolete_parse() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let transport = transport::Transport::controlled(listener.local_addr().unwrap());
    let row = |id: &str, open: &str, close: &str| {
        format!(
            r#"{{"ticker":"KXBTC15M-{id}","event_ticker":"KXBTC15M-E","open_time":"1970-01-01T{open}Z","close_time":"1970-01-01T{close}Z","status":"active"}}"#
        )
    };
    let m = row("M", "00:00:00", "00:15:00");
    let n = row("N", "00:15:00", "00:30:00");
    let page1 = format!(r#"{{"markets":[{m}],"cursor":"opaque+/="}}"#);
    let page2 = format!(r#"{{"markets":[{n}],"cursor":""}}"#);
    let detail_m = format!(r#"{{"market":{m}}}"#);
    let detail_n = format!(r#"{{"market":{n}}}"#);
    let peer = tokio::spawn(async move {
        let mut paths = Vec::new();
        for body in [
            r#"{"exchange_active":true}"#.to_owned(),
            r#"{"series":{"ticker":"KXBTC15M"}}"#.to_owned(),
            page1.clone(),
            page2.clone(),
            detail_m.clone(),
            detail_n.clone(),
            page1,
            page2,
            detail_m,
            detail_n,
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
                assert!(request.len() < 8192);
            }
            let path = std::str::from_utf8(&request)
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_owned();
            paths.push(path);
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
        paths
    });
    let worker =
        SourceWorker::prepare(&all_source_strategy(), &[], transport.memory().clone()).unwrap();
    let mut owner = Coordinator::new();
    let mut handle = owner.start_sources(worker).unwrap();
    while handle.latest.borrow().is_none() {
        handle.latest.changed().await.unwrap();
    }
    let token = handle
        .latest
        .borrow()
        .as_ref()
        .unwrap()
        .snapshot()
        .unwrap()
        .streams
        .iter()
        .find(|(s, _, _)| *s == metten_engine::sources::Stream::Metadata)
        .unwrap()
        .2;
    let (_stop, mut stop) = watch::channel(false);
    let fence = Fence::new();
    for startup in [true, false] {
        let result = tokio::time::timeout(
            Duration::from_secs(12),
            discovery::pair(
                &transport,
                &mut stop,
                &fence,
                1,
                0,
                startup,
                &handle.command,
                token,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result.markets.len(), 2);
        assert_eq!(result.markets[1].open_ms, 900000);
    }
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap(), [10, 0, 0]);
    let paths = peer.await.unwrap();
    assert_eq!(paths.len(), 10);
    assert!(paths[3].ends_with("cursor=opaque%2B%2F%3D"));
    assert_eq!(&paths[2..6], &paths[6..10]);
    handle.schedule.send_replace(Some(ScheduleLoss {
        epoch: Some(0),
        reason: metten_engine::sources::Unavailable::MetadataChanged,
        terminal: false,
    }));
    tokio::time::timeout(SHUTDOWN, async {
        loop {
            let changed = handle
                .latest
                .borrow()
                .as_ref()
                .and_then(|v| v.snapshot().ok())
                .is_some_and(|v| {
                    v.streams.iter().any(|(s, _, t)| {
                        *s == metten_engine::sources::Stream::Metadata && *t != token
                    })
                });
            if changed {
                break;
            }
            handle.latest.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let (reply, receive) = tokio::sync::oneshot::channel();
    handle
        .command
        .send(SourceCommand::Metadata {
            route: Route::Series,
            bytes: transport
                .memory()
                .copy(br#"{"series":{"ticker":"KXBTC15M"}}"#)
                .unwrap(),
            epoch: 0,
            now: 1,
            token,
            reply,
        })
        .await
        .unwrap();
    assert!(matches!(receive.await.unwrap(), Err(Failure::Stopped)));
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.finished());
}

#[tokio::test]
async fn three_owned_provider_tasks_send_seven_initial_controls_without_extra_connections() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let transport = Arc::new(transport::Transport::controlled(
        listener.local_addr().unwrap(),
    ));
    let (log, mut logs) = mpsc::channel(8);
    let peer = tokio::spawn(async move {
        let mut children = tokio::task::JoinSet::new();
        for _ in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let log = log.clone();
            children.spawn(async move {
                let mut peek=[0;4096];let n=socket.peek(&mut peek).await.unwrap();
                if !std::str::from_utf8(&peek[..n]).unwrap().starts_with("GET /trade-api/ws/v2 ") {
                    use tokio::io::AsyncWriteExt;
                    socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
                    return;
                }
                let mut socket=tokio_tungstenite::accept_async(socket).await.unwrap();
                while let Some(Ok(Message::Text(text)))=socket.next().await {
                    let value:serde_json::Value=serde_json::from_str(&text).unwrap();
                    log.send(value.clone()).await.unwrap();
                    if value["cmd"]=="subscribe" {
                        let channel=value["params"]["channels"][0].as_str().unwrap();
                        let sid=match channel {"orderbook_delta"=>1,"trade"=>2,"cfbenchmarks_value"=>3,"market_lifecycle_v2"=>4,_=>panic!("unexpected channel")};
                        socket.send(Message::Text(format!(r#"{{"type":"subscribed","msg":{{"sid":{sid},"channel":"{channel}"}}}}"#).into())).await.unwrap();
                    }
                }
            });
        }
        while let Some(result) = children.join_next().await {
            result.unwrap();
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(20), listener.accept())
                .await
                .is_err()
        );
    });
    let worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        transport.memory().clone(),
    )
    .unwrap();
    let mut owner = Coordinator::new();
    let mut handle = owner.start_sources(worker).unwrap();
    for provider in [Provider::Kalshi, Provider::Coinbase, Provider::Kraken] {
        owner
            .start_provider(
                provider,
                transport.clone(),
                &mut handle,
                if provider == Provider::Kalshi {
                    Some(("synthetic".into(), signing::Key::parse(&ed_pem()).unwrap()))
                } else {
                    None
                },
            )
            .unwrap();
    }
    let controls = tokio::time::timeout(Duration::from_secs(5), async {
        let mut values = Vec::new();
        for _ in 0..7 {
            values.push(logs.recv().await.unwrap());
        }
        values
    })
    .await
    .unwrap();
    assert_eq!(
        controls.iter().filter(|c| c["cmd"] == "subscribe").count(),
        4
    );
    assert_eq!(
        controls
            .iter()
            .filter(|c| c["params"]["action"] == "indexlist")
            .count(),
        1,
        "controls: {controls:?}"
    );
    assert_eq!(
        controls.iter().filter(|c| c["type"] == "subscribe").count(),
        1
    );
    assert_eq!(
        controls
            .iter()
            .filter(|c| c["method"] == "subscribe")
            .count(),
        1
    );
    for provider in [Provider::Kalshi, Provider::Coinbase, Provider::Kraken] {
        assert_eq!(transport.attempts(provider).unwrap()[1], 1);
    }
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.finished());
    peer.await.unwrap();
}

#[tokio::test]
async fn unpolled_owned_service_drop_revokes_its_session_and_releases_the_socket() {
    use futures_util::StreamExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        assert!(socket.next().await.is_none_or(|m| m.is_err()));
    });
    let fence = Fence::new();
    let connection = transport::local_connection(address, Provider::Coinbase, fence.clone()).await;
    let transport = transport::Transport::new(Environment::Demo).unwrap();
    let (_tx, mut controls) = mpsc::channel(1);
    let (tx, _) = intake();
    let (_stop, stop) = watch::channel(false);
    let source =
        SourceWorker::prepare(&all_source_strategy(), &[], transport.memory().clone()).unwrap();
    let future = transport.serve(connection, &mut controls, tx, source.tokens(), stop, None);
    drop(future);
    assert!(!fence.current(fence.token()));
    tokio::time::timeout(SHUTDOWN, peer).await.unwrap().unwrap();
}

#[tokio::test]
async fn explicit_recovery_reuses_owned_control_receiver_only_after_join() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let transport = Arc::new(transport::Transport::controlled(
        listener.local_addr().unwrap(),
    ));
    let peer = tokio::spawn(async move {
        for id in [10, 11] {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            assert!(socket.next().await.unwrap().unwrap().is_text());
            socket.send(Message::Text(format!(r#"{{"type":"last_match","product_id":"BTC-USD","trade_id":{id},"price":"68000.{id}","time":"t"}}"#).into())).await.unwrap();
            let _ = socket.next().await;
        }
    });
    let mut owner = Coordinator::new();
    let worker =
        SourceWorker::prepare(&all_source_strategy(), &[], transport.memory().clone()).unwrap();
    let mut handle = owner.start_sources(worker).unwrap();
    for id in [10, 11] {
        owner
            .start_provider(Provider::Coinbase, transport.clone(), &mut handle, None)
            .unwrap();
        tokio::time::timeout(SHUTDOWN, async {
            loop {
                let ready = handle
                    .latest
                    .borrow()
                    .as_ref()
                    .and_then(|v| v.snapshot().ok())
                    .is_some_and(|s| {
                        s.bindings.iter().any(|b| {
                            b.provider_identity.as_deref()
                                == Some(if id == 10 { "10" } else { "11" })
                                && b.initialized
                        })
                    });
                if ready {
                    break;
                }
                handle.latest.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        let old = handle.latest.borrow().clone().unwrap();
        owner
            .stop_provider(Provider::Coinbase, Instant::now() + SHUTDOWN)
            .await
            .unwrap();
        assert!(!old.stream_current(metten_engine::sources::Stream::Coinbase));
        owner.recover_provider(Provider::Coinbase, false).unwrap();
    }
    assert_eq!(transport.attempts(Provider::Coinbase).unwrap(), [0, 2, 2]);
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    peer.await.unwrap();
}

#[test]
fn repeated_native_rollover_retains_two_books_and_reuses_shared_subscription() {
    check_native_rollover(0);
}

#[test]
fn prepared_rollover_crosses_opening_between_worker_ticks_once_at_scheduled_time() {
    for delay in [1, 2, 5] {
        check_native_rollover(delay);
    }
}

#[test]
fn missed_opening_never_backdates_late_readiness_or_revoked_preparation() {
    use metten_engine::{sources::*, strategy::EvidenceChange};
    for rebuild in [false, true] {
        let (mut worker, fence) = prepared_rollover_worker();
        worker.advance(899999, |_| Ok(())).unwrap();
        worker
            .state
            .invalidate(Scope::Market, Unavailable::MetadataChanged, 899999, None)
            .unwrap();
        if rebuild {
            worker
                .install_markets(
                    &source_markets()[1..],
                    worker.tokens()[Stream::Metadata as usize],
                    900001,
                    |_| Ok(()),
                )
                .unwrap();
            feed_source(
                &mut worker,
                &fence,
                super::Provider::Kalshi,
                book_frame(3, "KXBTC15M-1", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#).as_bytes(),
                900001,
                None,
            )
            .unwrap();
        }
        let mut entries = 0;
        worker
            .advance(900002, |effect| {
                entries += effect
                    .batch
                    .transactions
                    .iter()
                    .flat_map(|t| &t.changes)
                    .filter(|c| matches!(c, EvidenceChange::MarketBoundary { .. }))
                    .count();
                Ok(())
            })
            .unwrap();
        assert_eq!(entries, 0);
        let view = worker.state.snapshot(900002).unwrap();
        assert_eq!(
            view.current_market.as_deref(),
            rebuild.then_some("KXBTC15M-1")
        );
        assert_eq!(view.markets[0].preparation == Preparation::Ready, rebuild);
    }
}

fn prepared_rollover_worker() -> (SourceWorker, Fence) {
    use metten_engine::{sources::*, strategy::*};
    let strategy =
        ValidatedStrategy::parse(include_str!("../../../examples/learning.json")).unwrap();
    let memory = memory::Memory::default();
    let prepared = SourceWorker::prepare(&strategy, &source_markets(), memory.clone()).unwrap();
    let bindings: Vec<_> = prepared
        .state
        .snapshot(0)
        .unwrap()
        .bindings
        .into_iter()
        .map(|b| b.binding)
        .collect();
    drop(prepared);
    let caps: Vec<_> = bindings
        .iter()
        .map(|b| {
            (
                b.binding_id,
                Capabilities {
                    current: None,
                    observations: None,
                    history: None,
                },
            )
        })
        .collect();
    let state = SourceState::new(&strategy, &source_markets(), &bindings, &caps).unwrap();
    let mut worker = SourceWorker::new(state, memory).unwrap();
    worker.lease = Some(10_000_000);
    let fence = Fence::new();
    worker
        .connection(
            super::Provider::Kalshi,
            worker.tokens()[0],
            Connection::Open,
            0,
        )
        .unwrap();
    feed_source(
        &mut worker,
        &fence,
        super::Provider::Kalshi,
        br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#,
        0,
        None,
    )
    .unwrap();
    worker
        .install_markets(
            &source_markets(),
            worker.tokens()[Stream::Metadata as usize],
            0,
            |_| Ok(()),
        )
        .unwrap();
    for (seq, id) in [(1, 0), (2, 1)] {
        feed_source(
            &mut worker,
            &fence,
            super::Provider::Kalshi,
            book_frame(
                seq,
                &format!("KXBTC15M-{id}"),
                r#"[["0.36","1"]]"#,
                r#"[["0.64","1"]]"#,
            )
            .as_bytes(),
            0,
            None,
        )
        .unwrap();
    }
    (worker, fence)
}

fn check_native_rollover(delay: u64) {
    use metten_engine::{sources::*, strategy::*};
    let (mut worker, fence) = prepared_rollover_worker();
    let (controls, mut requests) = mpsc::channel(7);
    worker.controls[0] = Some(controls);
    let mut seq = 2;
    for id in 1..=3 {
        let opening = id * 900000;
        let at = opening + delay;
        worker.advance(opening - 1, |_| Ok(())).unwrap();
        let before = worker.tokens();
        let mut entries = 0;
        worker
            .advance(at, |effect| {
                if effect.batch.transactions.iter().any(|t| {
                    t.changes
                        .iter()
                        .any(|c| matches!(c, EvidenceChange::MarketBoundary { .. }))
                }) {
                    assert_eq!(effect.batch.target_ms, opening);
                    assert!(effect.batch.transactions.iter().all(|t| t.at_ms == opening));
                }
                entries += effect
                    .batch
                    .transactions
                    .iter()
                    .flat_map(|t| &t.changes)
                    .filter(|c| matches!(c, EvidenceChange::MarketBoundary { .. }))
                    .count();
                Ok(())
            })
            .unwrap();
        assert_eq!(entries, 1);
        assert_eq!(
            worker.state.snapshot(at).unwrap().current_market.as_deref(),
            Some(format!("KXBTC15M-{id}").as_str())
        );
        assert!(matches!(
            requests.try_recv().unwrap(),
            Control::KalshiBookUpdate {
                action: BookAction::Delete,
                ..
            }
        ));
        assert!(requests.try_recv().is_err());
        let next = MarketDescriptor {
            series_id: "KXBTC15M".into(),
            market_id: format!("KXBTC15M-{}", id + 1),
            contract_id: format!("contract-{}", id + 1),
            open_ms: (opening + 900000) as i64,
            close_ms: (opening + 1800000) as i64,
        };
        worker
            .install_markets(
                std::slice::from_ref(&next),
                worker.tokens()[Stream::Metadata as usize],
                at,
                |_| Ok(()),
            )
            .unwrap();
        assert!(matches!(
            requests.try_recv().unwrap(),
            Control::KalshiBookUpdate {
                sid: 1,
                action: BookAction::Add,
                ..
            }
        ));
        seq += 1;
        feed_source(
            &mut worker,
            &fence,
            super::Provider::Kalshi,
            book_frame(
                seq,
                &next.market_id,
                r#"[["0.36","1"]]"#,
                r#"[["0.64","1"]]"#,
            )
            .as_bytes(),
            at,
            None,
        )
        .unwrap();
        let snapshot = worker.state.snapshot(at).unwrap();
        assert_eq!(snapshot.markets.len(), 2);
        assert_eq!(snapshot.bindings.len(), 4);
        assert_eq!(worker.tokens(), before);
        assert!(fence.current(fence.token()));
        let mut duplicates = 0;
        worker
            .advance(at, |e| {
                duplicates += e
                    .batch
                    .transactions
                    .iter()
                    .flat_map(|t| &t.changes)
                    .filter(|c| matches!(c, EvidenceChange::MarketBoundary { .. }))
                    .count();
                Ok(())
            })
            .unwrap();
        assert_eq!(duplicates, 0);
    }
}

#[tokio::test]
async fn same_socket_recovery_discards_in_progress_old_receive_then_restores_both_books() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let transport = Arc::new(transport::Transport::new(Environment::Demo).unwrap());
    let (step, mut steps) = mpsc::channel(1);
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        assert!(socket.next().await.unwrap().unwrap().is_text());
        for frame in [
            r#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#.to_owned(),
            book_frame(1, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#),
            book_frame(2, "KXBTC15M-1", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#),
            book_frame(4, "KXBTC15M-0", "[]", "[]"),
        ] {
            socket.send(Message::Text(frame.into())).await.unwrap();
            steps.recv().await.unwrap();
        }
        let recovery = socket.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(recovery.contains("get_snapshot"));
        // This first response is returned by the receive already pending before recovery began.
        socket
            .send(Message::Text(
                book_frame(5, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#).into(),
            ))
            .await
            .unwrap();
        steps.recv().await.unwrap();
        let repeated = socket.next().await.unwrap().unwrap().into_text().unwrap();
        assert!(repeated.contains("get_snapshot"));
        for (seq, id) in [(6, 0), (7, 1)] {
            socket
                .send(Message::Text(
                    book_frame(
                        seq,
                        &format!("KXBTC15M-{id}"),
                        r#"[["0.36","1"]]"#,
                        r#"[["0.64","1"]]"#,
                    )
                    .into(),
                ))
                .await
                .unwrap();
            steps.recv().await.unwrap();
        }
        let _ = socket.next().await;
    });
    let mut worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        transport.memory().clone(),
    )
    .unwrap();
    worker
        .connection(
            Provider::Kalshi,
            worker.tokens()[0],
            metten_engine::sources::Connection::Open,
            0,
        )
        .unwrap();
    let tokens = worker.tokens();
    let fence = Fence::new();
    let connection = transport::local_connection(address, Provider::Kalshi, fence.clone()).await;
    let (controls, mut controls_rx) = mpsc::channel(7);
    worker.controls[0] = Some(controls.clone());
    controls
        .send(Control::KalshiSubscribe {
            id: 1,
            channel: Channel::Book,
            markets: source_markets().into_iter().map(|m| m.market_id).collect(),
        })
        .await
        .unwrap();
    let (intake, mut receipts) = intake();
    let (stop, stopped) = watch::channel(false);
    let progress = watch::channel(transport::Progress::new()).0;
    let service_transport = transport.clone();
    let service = tokio::spawn(async move {
        service_transport
            .serve(
                connection,
                &mut controls_rx,
                intake,
                tokens,
                stopped,
                Some(progress),
            )
            .await
    });
    for i in 0..7 {
        let mut receipt = tokio::time::timeout(SHUTDOWN, receipts.recv())
            .await
            .unwrap()
            .unwrap();
        receipt.receipt_ms = Some(0);
        let result = worker.process(receipt, 0, 0, |_| Ok(()));
        if i == 4 {
            assert_eq!(result, Err(Failure::Stopped))
        } else {
            result.unwrap();
        }
        step.send(()).await.unwrap();
    }
    assert!(fence.current(fence.token()));
    let initialized = worker
        .state
        .snapshot(0)
        .unwrap()
        .bindings
        .into_iter()
        .filter(|b| b.requirement.field == "buyable_ask" && b.initialized)
        .count();
    assert_eq!(initialized, 4);
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap(), [0, 0, 3]);
    stop.send_replace(true);
    assert_eq!(
        tokio::time::timeout(SHUTDOWN, service)
            .await
            .unwrap()
            .unwrap(),
        Err(Failure::Stopped)
    );
    peer.await.unwrap();
}

#[tokio::test]
async fn source_context_charge_lives_until_its_retained_control_handle_is_dropped() {
    let memory = memory::Memory::default();
    let worker = SourceWorker::prepare(&all_source_strategy(), &[], memory.clone()).unwrap();
    let mut owner = Coordinator::new();
    let mut handle = owner.start_sources(worker).unwrap();
    while handle.latest.borrow().is_none() {
        handle.latest.changed().await.unwrap();
    }
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.finished());
    assert!(memory.used() >= 64 * 1024);
    drop(handle);
    assert_eq!(memory.used(), 0);
}

#[test]
fn metadata_revocation_discards_book_baseline_before_new_generation_deltas() {
    use metten_engine::sources::{Connection, Stream};
    let mut worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        memory::Memory::default(),
    )
    .unwrap();
    let fence = Fence::new();
    worker
        .connection(Provider::Kalshi, worker.tokens()[0], Connection::Open, 0)
        .unwrap();
    for (sid, channel) in [(1, "orderbook_delta"), (4, "market_lifecycle_v2")] {
        feed_source(
            &mut worker,
            &fence,
            Provider::Kalshi,
            format!(r#"{{"type":"subscribed","msg":{{"sid":{sid},"channel":"{channel}"}}}}"#)
                .as_bytes(),
            0,
            None,
        )
        .unwrap();
    }
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        book_frame(1, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#).as_bytes(),
        0,
        None,
    )
    .unwrap();
    let before = worker.snapshot(0).unwrap();
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        br#"{"type":"market_lifecycle_v2","sid":4,"seq":1,"msg":{"market_ticker":"KXBTC15M-0"}}"#,
        0,
        None,
    )
    .unwrap();
    assert!(!before.stream_current(Stream::Book));
    assert!(!before.stream_current(Stream::Metadata));
    assert!(before.stream_current(Stream::Brti));
    feed_source(&mut worker,&fence,Provider::Kalshi,br#"{"type":"orderbook_delta","sid":1,"seq":2,"msg":{"market_ticker":"KXBTC15M-0","side":"yes","price_dollars":"0.36","delta_fp":"1"}}"#,0,None).unwrap();
    assert!(
        worker
            .state
            .snapshot(0)
            .unwrap()
            .bindings
            .iter()
            .filter(|b| b.requirement.field == "buyable_ask")
            .all(|b| !b.initialized)
    );
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        book_frame(3, "KXBTC15M-0", r#"[["0.40","1"]]"#, r#"[["0.60","1"]]"#).as_bytes(),
        0,
        None,
    )
    .unwrap();
    assert_eq!(
        worker
            .state
            .snapshot(0)
            .unwrap()
            .bindings
            .iter()
            .filter(|b| b.requirement.field == "buyable_ask" && b.initialized)
            .count(),
        2
    );
}

#[test]
fn current_next_details_are_scoped_and_retired_metadata_cannot_reopen_a_slot() {
    use metten_engine::sources::{Connection, Stream};
    let mut worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        memory::Memory::default(),
    )
    .unwrap();
    let fence = Fence::new();
    worker
        .connection(Provider::Kalshi, worker.tokens()[0], Connection::Open, 0)
        .unwrap();
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#,
        0,
        None,
    )
    .unwrap();
    for (seq, id, qty) in [(1, 0, "2"), (2, 1, "7")] {
        let no = format!(r#"[["0.64","{qty}"]]"#);
        feed_source(
            &mut worker,
            &fence,
            Provider::Kalshi,
            book_frame(seq, &format!("KXBTC15M-{id}"), r#"[["0.36","1"]]"#, &no).as_bytes(),
            0,
            None,
        )
        .unwrap();
    }
    let before = worker.snapshot(0).unwrap();
    for (id, want) in [(0, 2_000_000), (1, 7_000_000)] {
        let Some(wire::Details::Book { quantities, .. }) = before
            .details(Stream::Book, Some(&format!("KXBTC15M-{id}")))
            .unwrap()
        else {
            panic!("book details")
        };
        assert_eq!(quantities[2].unwrap().millionths(), want);
    }
    assert!(matches!(
        before.details(Stream::Book, None),
        Err(Failure::InvalidRequest)
    ));
    worker.advance(900000, |_| Ok(())).unwrap();
    assert!(!before.stream_current(Stream::Book));
    assert_eq!(
        worker.install_markets(
            &source_markets(),
            worker.tokens()[Stream::Metadata as usize],
            900000,
            |_| Ok(())
        ),
        Err(Failure::Stopped)
    );
    let now = worker.snapshot(900000).unwrap();
    assert!(
        now.details(Stream::Book, Some("KXBTC15M-0"))
            .unwrap()
            .is_none()
    );
    assert!(
        now.details(Stream::Book, Some("KXBTC15M-1"))
            .unwrap()
            .is_some()
    );
    assert_eq!(now.snapshot().unwrap().markets.len(), 1);
}

#[test]
fn native_stop_revokes_a_pending_controlled_consumer_before_more_ordinary_work() {
    use metten_engine::strategy::*;
    let strategy =
        ValidatedStrategy::parse(include_str!("../../../examples/learning.json")).unwrap();
    let markets = source_markets();
    let memory = memory::Memory::default();
    let mut worker = SourceWorker::prepare(&strategy, &markets, memory.clone()).unwrap();
    let bindings: Vec<_> = worker
        .state
        .snapshot(0)
        .unwrap()
        .bindings
        .into_iter()
        .map(|b| b.binding)
        .collect();
    let ids: Vec<_> = bindings.iter().map(|b| b.binding_id).collect();
    let mut evaluator = Evaluator::new(
        strategy,
        RunContext {
            strategy_version: "controlled".into(),
            run_id: "pending-stop".into(),
            go_time: "controlled".into(),
            series_id: "KXBTC15M".into(),
            markets,
            bindings,
        },
    )
    .unwrap();
    worker
        .connection(
            Provider::Kalshi,
            worker.tokens()[0],
            metten_engine::sources::Connection::Open,
            0,
        )
        .unwrap();
    let fence = Fence::new();
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#,
        0,
        None,
    )
    .unwrap();
    let bytes = book_frame(1, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#);
    let mut item = Intake::new(
        Provider::Kalshi,
        &fence,
        memory.copy(bytes.as_bytes()).unwrap(),
        Instant::now(),
    );
    item.receipt_ms = Some(0);
    item.source_tokens = Some(worker.tokens());
    worker
        .process(item, 0, 0, |effect| {
            evaluator.begin(effect.batch).unwrap();
            Ok(())
        })
        .unwrap();
    let stamp = worker.state.stamp(&ids).unwrap();
    let view = worker.snapshot(0).unwrap();
    let tokens = worker.tokens();
    fence.revoke();
    assert!(!view.stream_current(metten_engine::sources::Stream::Book));
    let mut late = Intake::new(
        Provider::Kalshi,
        &fence,
        memory.copy(bytes.as_bytes()).unwrap(),
        Instant::now(),
    );
    late.source_tokens = Some(tokens);
    late.receipt_ms = Some(0);
    assert_eq!(
        worker.process(late, 0, 0, |_| panic!(
            "no second batch during pending Stop"
        )),
        Err(Failure::Stopped)
    );
    worker.observe_stops(0).unwrap();
    assert!(!worker.state.is_current(&stamp));
    assert!(matches!(
        evaluator.process_slice(SliceControl::Stop, None),
        SliceResult::Stopped
    ));
    assert!(matches!(
        evaluator.process_slice(SliceControl::Continue, None),
        SliceResult::Stopped
    ));
}

#[tokio::test]
async fn ping_burst_replies_do_not_wait_for_a_subscription_permit() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let transport = Arc::new(transport::Transport::controlled(
        listener.local_addr().unwrap(),
    ));
    let (observed, receive) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        assert!(socket.next().await.unwrap().unwrap().is_text());
        socket.send(Message::Ping(vec![1].into())).await.unwrap();
        socket.send(Message::Ping(vec![2].into())).await.unwrap();
        tokio::time::timeout(Duration::from_millis(250), async {
            loop {
                match socket.next().await.unwrap().unwrap() {
                    Message::Pong(bytes) if bytes.as_ref() == [2] => break,
                    Message::Pong(_) => (),
                    other => panic!("pong must precede the next rate-limited control: {other:?}"),
                }
            }
        })
        .await
        .unwrap();
        observed.send(()).unwrap();
        let _ = socket.next().await;
    });
    let fence = Fence::new();
    let (_initial, mut initial) = watch::channel(false);
    let connection = transport
        .connect(
            Provider::Coinbase,
            None,
            &mut initial,
            Instant::now() + SHUTDOWN,
            &fence,
        )
        .await
        .unwrap();
    let worker =
        SourceWorker::prepare(&all_source_strategy(), &[], transport.memory().clone()).unwrap();
    let tokens = worker.tokens();
    let (controls, mut controls_rx) = mpsc::channel(2);
    controls
        .send(Control::Coinbase { subscribe: true })
        .await
        .unwrap();
    controls
        .send(Control::Coinbase { subscribe: false })
        .await
        .unwrap();
    let (intake, _) = intake();
    let (stop, stopped) = watch::channel(false);
    let service_transport = transport.clone();
    let service = tokio::spawn(async move {
        service_transport
            .serve(connection, &mut controls_rx, intake, tokens, stopped, None)
            .await
    });
    tokio::time::timeout(SHUTDOWN, receive)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(transport.attempts(Provider::Coinbase).unwrap(), [0, 1, 1]);
    stop.send_replace(true);
    assert_eq!(
        tokio::time::timeout(SHUTDOWN, service)
            .await
            .unwrap()
            .unwrap(),
        Err(Failure::Stopped)
    );
    peer.await.unwrap();
}

#[tokio::test]
async fn delayed_handshake_subscribes_only_markets_still_retained_after_upgrade() {
    use futures_util::StreamExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let transport = Arc::new(transport::Transport::controlled(
        listener.local_addr().unwrap(),
    ));
    let (accepted, accept) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let (observed, received) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        accepted.send(()).unwrap();
        released.await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        let command = socket.next().await.unwrap().unwrap();
        let command: serde_json::Value = serde_json::from_str(command.to_text().unwrap()).unwrap();
        assert_eq!(
            command["params"]["channels"],
            serde_json::json!(["orderbook_delta"])
        );
        assert_eq!(
            command["params"]["market_tickers"],
            serde_json::json!(["KXBTC15M-1"])
        );
        observed.send(()).unwrap();
        while socket.next().await.is_some_and(|m| m.is_ok()) {}
    });
    let mut markets = source_markets();
    for market in &mut markets {
        market.open_ms -= 899_000;
        market.close_ms -= 899_000;
    }
    let worker =
        SourceWorker::prepare(&all_source_strategy(), &markets, transport.memory().clone())
            .unwrap();
    let mut owner = Coordinator::new();
    let mut handle = owner.start_sources(worker).unwrap();
    owner
        .start_provider(
            Provider::Kalshi,
            transport.clone(),
            &mut handle,
            Some(("synthetic".into(), signing::Key::parse(&ed_pem()).unwrap())),
        )
        .unwrap();
    tokio::time::timeout(SHUTDOWN, accept)
        .await
        .unwrap()
        .unwrap();
    assert!(
        handle
            .latest
            .borrow()
            .as_ref()
            .unwrap()
            .snapshot()
            .unwrap()
            .markets
            .iter()
            .any(|m| m.descriptor.market_id == "KXBTC15M-0")
    );
    tokio::time::timeout(SHUTDOWN, async {
        loop {
            if handle.latest.borrow().as_ref().is_some_and(|v| {
                v.snapshot().is_ok_and(|v| {
                    v.markets
                        .iter()
                        .all(|m| m.descriptor.market_id != "KXBTC15M-0")
                })
            }) {
                break;
            }
            handle.latest.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    release.send(()).unwrap();
    tokio::time::timeout(SHUTDOWN, received)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap()[1], 1);
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    peer.await.unwrap();
}

#[test]
fn fixed_display_reads_are_local_and_revoked_optional_sources_stay_scoped() {
    let transport = transport::Transport::new(Environment::Demo).unwrap();
    let mut worker = SourceWorker::prepare_display(transport.memory().clone()).unwrap();
    assert!(worker.membership([true, true, true, true], 0).is_ok());
    let view = worker.snapshot(0).unwrap();
    for _ in 0..20 {
        let value = view.wire_snapshot(10);
        assert!(value["owner"].is_string() && value["revision"].is_string());
        assert!(
            value["bindings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|b| b["usable"] == false && b["history_ready"] == false)
        );
        for i in 0..4 {
            view.wire_details(i, 10).unwrap();
        }
    }
    for provider in [Provider::Kalshi, Provider::Coinbase, Provider::Kraken] {
        assert_eq!(transport.attempts(provider).unwrap(), [0, 0, 0]);
    }
    worker.membership([false, true, true, true], 1).unwrap();
    assert!(!view.stream_current(metten_engine::sources::Stream::Book));
    assert!(view.stream_current(metten_engine::sources::Stream::Brti));
}
#[tokio::test]
async fn isolated_transport_refuses_production_routes_before_spending() {
    let transport = transport::Transport::isolated().unwrap();
    let fence = Fence::new();
    let (_send, mut stop) = watch::channel(false);
    assert!(matches!(
        transport
            .get(
                &Route::ExchangeStatus,
                &mut stop,
                Instant::now() + SHUTDOWN,
                &fence
            )
            .await,
        Err(Failure::InvalidRequest)
    ));
    assert!(matches!(
        transport
            .connect(
                Provider::Coinbase,
                None,
                &mut stop,
                Instant::now() + SHUTDOWN,
                &fence
            )
            .await,
        Err(Failure::InvalidRequest)
    ));
    assert_eq!(transport.attempts(Provider::Kalshi).unwrap(), [0, 0, 0]);
    assert_eq!(transport.attempts(Provider::Coinbase).unwrap(), [0, 0, 0]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixture_dispatch_completes_only_after_real_decode_and_publication() {
    let transport = Arc::new(transport::Transport::isolated().unwrap());
    let mut owner = Coordinator::new();
    let mut handle = owner
        .start_sources(SourceWorker::prepare_display(transport.memory().clone()).unwrap())
        .unwrap();
    let disconnected = handle.clone();
    assert!(
        tokio::task::spawn_blocking(
            move || disconnected.dispatch_fixture(Provider::Coinbase, b"{}")
        )
        .await
        .unwrap()
        .is_err()
    );
    handle.enable(2, true).unwrap();
    owner
        .start_provider(Provider::Coinbase, transport.clone(), &mut handle, None)
        .unwrap();
    tokio::time::timeout(SHUTDOWN, async {
        loop {
            if handle.latest.borrow().as_ref().is_some_and(|v| {
                v.wire_snapshot(handle.elapsed_ms())["sources"][2]["connection"] == "open"
            }) {
                break;
            }
            handle.latest.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let receipt = handle.clone();
    tokio::task::spawn_blocking(move||receipt.dispatch_fixture(Provider::Coinbase,br#"{"type":"last_match","product_id":"BTC-USD","trade_id":10,"price":"68000.12","time":"2026-11-16T00:00:00Z"}"#)).await.unwrap().unwrap();
    let view = handle.latest.borrow().clone().unwrap();
    let before = view.wire_snapshot(handle.elapsed_ms());
    assert_eq!(before["bindings"][1]["value"], "68000.12");
    let invalid = handle.clone();
    tokio::task::spawn_blocking(move || invalid.dispatch_fixture(Provider::Coinbase, b"not json"))
        .await
        .unwrap()
        .unwrap();
    let after = handle
        .latest
        .borrow()
        .clone()
        .unwrap()
        .wire_snapshot(handle.elapsed_ms());
    assert_eq!(after["bindings"][1]["usable"], false);
    assert_ne!(after["revision"], before["revision"]);
    assert!(!view.stream_current(metten_engine::sources::Stream::Coinbase));
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
    assert!(owner.finished());
}

#[test]
fn source_failure_reason_survives_disconnect_publication_without_reviving_old_views() {
    use metten_engine::sources::{Connection, Stream};
    for good_first in [false, true] {
        let memory = memory::Memory::default();
        let mut worker = SourceWorker::prepare_display(memory).unwrap();
        worker.membership([false, false, true, true], 0).unwrap();
        let fence = Fence::new();
        worker
            .connection(
                Provider::Coinbase,
                worker.state.token(Stream::Coinbase),
                Connection::Open,
                0,
            )
            .unwrap();
        if good_first {
            feed_source(&mut worker, &fence, Provider::Coinbase, br#"{"type":"last_match","product_id":"BTC-USD","trade_id":10,"price":"68000.12","time":"2026-11-16T00:00:00Z"}"#, 0, None).unwrap();
        }
        let before = worker.snapshot(0).unwrap();
        assert_eq!(
            feed_source(
                &mut worker,
                &fence,
                Provider::Coinbase,
                b"not json",
                1,
                None
            ),
            Err(Failure::Protocol)
        );
        let published = worker.snapshot(1).unwrap().wire_snapshot(1);
        assert_eq!(published["bindings"][1]["reason"], "invalid_value");
        assert_eq!(published["bindings"][1]["usable"], false);
        assert_eq!(
            before.wire_snapshot(1)["bindings"][1]["reason"],
            "disconnected"
        );
        worker.observe_stops(2).unwrap();
        let after = worker.snapshot(2).unwrap();
        let later = after.wire_snapshot(2);
        assert_eq!(later["bindings"][1]["reason"], "invalid_value");
        assert_eq!(later["bindings"][1]["value"].is_null(), !good_first);
        assert_eq!(later["bindings"][1]["usable"], false);
        assert!(after.stream_current(Stream::Kraken));
        worker.membership([false, false, false, true], 3).unwrap();
        assert_eq!(
            after.wire_snapshot(3)["bindings"][1]["reason"],
            "disconnected"
        );
        assert_eq!(
            worker.snapshot(3).unwrap().wire_snapshot(3)["bindings"][1]["reason"],
            "disconnected"
        );
        worker.membership([false, false, true, true], 4).unwrap();
        worker
            .connection(
                Provider::Coinbase,
                worker.state.token(Stream::Coinbase),
                Connection::Open,
                4,
            )
            .unwrap();
        assert_ne!(
            worker.snapshot(4).unwrap().wire_snapshot(4)["bindings"][1]["reason"],
            "invalid_value"
        );
        let fresh = Fence::new();
        feed_source(&mut worker, &fresh, Provider::Coinbase, br#"{"type":"last_match","product_id":"BTC-USD","trade_id":11,"price":"68000.12","time":"2026-11-16T00:00:01Z"}"#, 4, None).unwrap();
        let current = worker.snapshot(4).unwrap();
        fresh.revoke();
        assert_eq!(
            current.wire_snapshot(4)["bindings"][1]["reason"],
            "disconnected"
        );
    }
}

#[test]
fn logical_kalshi_retirement_preserves_brti_and_releases_old_subscription_ids() {
    use metten_engine::sources::{Connection, Stream};
    let memory = memory::Memory::default();
    let mut worker =
        SourceWorker::prepare(&all_source_strategy(), &source_markets(), memory.clone()).unwrap();
    let (outbound, mut controls) = mpsc::channel(7);
    worker.controls[0] = Some(outbound);
    let fence = Fence::new();
    let progress = watch::channel(transport::Progress::new()).0;
    let feed = |worker: &mut SourceWorker, bytes: &[u8], at: u64| {
        let mut item = Intake::new(
            Provider::Kalshi,
            &fence,
            memory.copy(bytes).unwrap(),
            Instant::now(),
        );
        item.source_tokens = Some(worker.tokens());
        item.receipt_ms = Some(at);
        item.progress = Some(progress.clone());
        worker.process(item, at, 0, |_| Ok(()))
    };
    worker
        .connection(Provider::Kalshi, worker.tokens()[0], Connection::Open, 0)
        .unwrap();
    feed(
        &mut worker,
        br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#,
        1,
    )
    .unwrap();
    feed(
        &mut worker,
        br#"{"type":"subscribed","msg":{"sid":3,"channel":"cfbenchmarks_value"}}"#,
        2,
    )
    .unwrap();
    while controls.try_recv().is_ok() {}
    let old = worker.snapshot(2).unwrap();
    worker.membership([false, true, true, true], 3).unwrap();
    assert!(!old.stream_current(Stream::Book));
    assert!(old.stream_current(Stream::Brti));
    assert!(
        matches!(controls.try_recv().unwrap(), Control::KalshiUnsubscribe { subscriptions, .. } if subscriptions == [1])
    );
    assert!(controls.try_recv().is_err());
    assert_eq!(
        feed(
            &mut worker,
            book_frame(2, "KXBTC15M-0", "[]", "[]").as_bytes(),
            4
        ),
        Err(Failure::Stopped)
    );
    assert!(
        progress.borrow().recovery.iter().all(Option::is_none),
        "retired prices cannot request snapshots"
    );
    assert!(controls.try_recv().is_err());
    worker.membership([true, true, true, true], 5).unwrap();
    while let Ok(control) = controls.try_recv() {
        assert!(
            !matches!(
                control,
                Control::KalshiSubscribe {
                    channel: Channel::Book,
                    ..
                }
            ),
            "book subscribe waits for old unsubscribe completion"
        );
    }
    assert_eq!(
        feed(
            &mut worker,
            book_frame(3, "KXBTC15M-0", "[]", "[]").as_bytes(),
            6
        ),
        Err(Failure::Stopped)
    );
    assert!(progress.borrow().recovery.iter().all(Option::is_none));
    feed(
        &mut worker,
        br#"{"type":"unsubscribed","sid":1,"seq":4}"#,
        7,
    )
    .unwrap();
    assert!(matches!(
        controls.try_recv().unwrap(),
        Control::KalshiSubscribe {
            channel: Channel::Book,
            ..
        }
    ));
    assert!(controls.try_recv().is_err());
    feed(
        &mut worker,
        br#"{"type":"subscribed","msg":{"sid":9,"channel":"orderbook_delta"}}"#,
        8,
    )
    .unwrap();
    feed(
        &mut worker,
        book_frame(1, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#)
            .replace("\"sid\":1", "\"sid\":9")
            .as_bytes(),
        9,
    )
    .unwrap();
    assert!(fence.current(fence.token()));
    assert!(worker.snapshot(9).unwrap().stream_current(Stream::Brti));
    assert!(
        worker
            .state
            .snapshot(9)
            .unwrap()
            .bindings
            .iter()
            .any(|b| b.requirement.field == "buyable_ask" && b.initialized)
    );
    worker.membership([false, true, true, true], 10).unwrap();
    worker.membership([true, true, true, true], 11).unwrap();
    worker.membership([false, true, true, true], 12).unwrap();
    while let Ok(control) = controls.try_recv() {
        assert!(!matches!(
            control,
            Control::KalshiSubscribe {
                channel: Channel::Book,
                ..
            }
        ));
    }
    feed(
        &mut worker,
        br#"{"type":"unsubscribed","sid":9,"seq":2}"#,
        13,
    )
    .unwrap();
    assert!(
        controls.try_recv().is_err(),
        "a cancelled reconnect stays disconnected"
    );
    assert!(worker.snapshot(13).unwrap().stream_current(Stream::Brti));
}

#[test]
fn provider_ended_brti_subscription_preserves_the_book_without_command_identity() {
    use metten_engine::sources::{Connection, Stream};
    for code in [10, 25] {
        let memory = memory::Memory::default();
        let mut worker =
            SourceWorker::prepare(&all_source_strategy(), &source_markets(), memory.clone())
                .unwrap();
        let fence = Fence::new();
        worker
            .connection(Provider::Kalshi, worker.tokens()[0], Connection::Open, 0)
            .unwrap();
        for (sid, channel) in [(1, "orderbook_delta"), (3, "cfbenchmarks_value")] {
            feed_source(
                &mut worker,
                &fence,
                Provider::Kalshi,
                format!(r#"{{"type":"subscribed","msg":{{"sid":{sid},"channel":"{channel}"}}}}"#)
                    .as_bytes(),
                0,
                None,
            )
            .unwrap();
        }
        feed_source(
            &mut worker,
            &fence,
            Provider::Kalshi,
            book_frame(1, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#).as_bytes(),
            0,
            None,
        )
        .unwrap();
        let error = format!(
            r#"{{"type":"error","sid":3,"seq":1,"msg":{{"code":{code},"msg":"stream ended"}}}}"#
        );
        for bytes in [
            error.as_bytes(),
            error.as_bytes(),
            br#"{"type":"unsubscribed","sid":3,"seq":2}"#,
        ] {
            feed_source(&mut worker, &fence, Provider::Kalshi, bytes, 1, None).unwrap();
            let view = worker.snapshot(1).unwrap();
            assert!(fence.current(fence.token()));
            assert!(view.stream_current(Stream::Book));
            assert!(
                view.wire_snapshot(1)["bindings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|b| b["source"] == "brti")
                    .all(|b| b["reason"] == "disconnected")
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_cancels_pending_connect_and_cannot_be_coalesced_into_old_permission() {
    let transport = transport::Transport::isolated().unwrap();
    let mut owner = Coordinator::new();
    let handle = owner
        .start_sources(SourceWorker::prepare_display(transport.memory().clone()).unwrap())
        .unwrap();
    handle.enable(2, true).unwrap();
    let before = handle.latest.borrow().clone().unwrap();
    let generation = before.wire_snapshot(handle.elapsed_ms())["bindings"][1]["generation"].clone();
    let pending = handle.intent_revision(2);
    handle.disable(2).unwrap();
    assert!(matches!(
        handle.enable_if_current(2, pending),
        Err(Failure::Stopped)
    ));
    handle
        .enable_if_current(2, handle.intent_revision(2))
        .unwrap();
    let after = handle
        .latest
        .borrow()
        .clone()
        .unwrap()
        .wire_snapshot(handle.elapsed_ms());
    assert_ne!(after["bindings"][1]["generation"], generation);
    assert!(!before.stream_current(metten_engine::sources::Stream::Coinbase));
    owner.shutdown(Instant::now() + SHUTDOWN).await.unwrap();
}

#[test]
fn intake_arrival_interrupts_idle_wait_without_lost_wake() {
    for arrive_before_wait in [true, false] {
        let (tx, mut rx) = intake();
        let (registered_tx, registered_rx) = std::sync::mpsc::channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let idle = Arc::new(IntakeWake::default());
            let waker = std::task::Waker::from(idle.clone());
            assert!(matches!(
                poll_intake(&mut rx, &waker),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            registered_tx.send(()).unwrap();
            ready_rx.recv().unwrap();
            idle.wait(Duration::from_secs(2));
            assert!(poll_intake(&mut rx, &waker).is_ok());
            done_tx.send(()).unwrap();
        });
        registered_rx.recv().unwrap();
        if !arrive_before_wait {
            ready_tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }
        let fence = Fence::new();
        admit(
            &tx,
            &fence,
            Intake::new(
                Provider::Coinbase,
                &fence,
                memory::Memory::default().copy(b"{}").unwrap(),
                Instant::now(),
            ),
        )
        .unwrap();
        if arrive_before_wait {
            ready_tx.send(()).unwrap();
        }
        let result = done_rx.recv_timeout(Duration::from_millis(500));
        worker.join().unwrap();
        assert!(
            result.is_ok(),
            "arrival must interrupt idle wait, including before park"
        );
    }
}

#[test]
fn intake_notification_survives_a_consumed_park_token() {
    let (ready, registered) = std::sync::mpsc::channel();
    let (notified, notification) = std::sync::mpsc::channel();
    let (done, completion) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let idle = Arc::new(IntakeWake::default());
        ready.send(idle.clone()).unwrap();
        notification.recv().unwrap();
        std::thread::park_timeout(Duration::ZERO);
        idle.wait(Duration::from_secs(2));
        done.send(()).unwrap();
    });
    let idle = registered.recv().unwrap();
    std::task::Waker::from(idle).wake_by_ref();
    notified.send(()).unwrap();
    let result = completion.recv_timeout(Duration::from_millis(500));
    worker.join().unwrap();
    assert!(
        result.is_ok(),
        "arrival flag must survive another wait consuming the park token"
    );
}

#[test]
fn kalshi_provenance_survives_real_decode_and_details_projection() {
    use metten_engine::sources::Connection;
    let mut worker = SourceWorker::prepare(
        &all_source_strategy(),
        &source_markets(),
        memory::Memory::default(),
    )
    .unwrap();
    let fence = Fence::new();
    worker
        .connection(Provider::Kalshi, worker.tokens()[0], Connection::Open, 0)
        .unwrap();
    let frames: &[&[u8]] = &[
        br#"{"type":"subscribed","msg":{"sid":1,"channel":"orderbook_delta"}}"#,
        br#"{"type":"orderbook_snapshot","sending_ts_ms":1700000000001,"sid":1,"seq":1,"msg":{"market_ticker":"KXBTC15M-0","clock":7,"yes_dollars_fp":[["0.36","2"]],"no_dollars_fp":[["0.64","3"]]}}"#,
        br#"{"type":"orderbook_delta","sending_ts_ms":1700000000003,"sid":1,"seq":2,"msg":{"market_ticker":"KXBTC15M-0","clock":8,"ts_ms":1700000000002,"side":"yes","price_dollars":"0.36","delta_fp":"1"}}"#,
        br#"{"type":"subscribed","msg":{"sid":2,"channel":"trade"}}"#,
        br#"{"type":"trade","sending_ts_ms":1700000000005,"sid":2,"seq":9007199254740993,"msg":{"market_ticker":"KXBTC15M-0","trade_id":"trade-1","ts_ms":1700000000004,"yes_price_dollars":"0.36","no_price_dollars":"0.64"}}"#,
        br#"{"type":"subscribed","msg":{"sid":3,"channel":"cfbenchmarks_value"}}"#,
        br#"{"type":"cfbenchmarks_value_indexlist","sid":3,"seq":1,"msg":{"index_ids":["BRTI"]}}"#,
        br#"{"type":"cfbenchmarks_value","sending_ts_ms":1700000000008,"sid":3,"seq":2,"msg":{"index_id":"BRTI","received_at":1700000000007,"data":"{\"id\":\"BRTI\",\"time\":1700000000006,\"value\":\"68000.12\"}"}}"#,
    ];
    for frame in frames {
        feed_source(&mut worker, &fence, Provider::Kalshi, frame, 0, None).unwrap();
    }
    let view = worker.snapshot(0).unwrap();
    let details = view.wire_details(0, 0).unwrap();
    let values = details["values"].as_array().unwrap();
    let book = values
        .iter()
        .find(|v| v["detail"]["kind"] == "book")
        .unwrap();
    assert_eq!(book["provider_time_utc"], "1700000000002");
    assert_eq!(book["detail"]["sending_ts_ms"], "1700000000003");
    assert_eq!(book["detail"]["sid"], "1");
    assert_eq!(book["detail"]["sequence"], "2");
    let trade = values
        .iter()
        .find(|v| v["detail"]["kind"] == "trade")
        .unwrap();
    assert_eq!(trade["provider_time_utc"], "1700000000004");
    assert_eq!(trade["detail"]["sending_ts_ms"], "1700000000005");
    assert_eq!(trade["detail"]["sid"], "2");
    assert_eq!(trade["detail"]["sequence"], "9007199254740993");
    let brti = view.wire_details(1, 0).unwrap();
    let brti = &brti["values"][0];
    assert_eq!(brti["provider_time_utc"], "1700000000006");
    assert_eq!(brti["detail"]["received_at"], "1700000000007");
    assert_eq!(brti["detail"]["sending_ts_ms"], "1700000000008");
    assert_eq!(brti["detail"]["sid"], "3");
    assert_eq!(brti["detail"]["sequence"], "2");
    assert!(
        view.wire_snapshot(0)["bindings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| b["usable"] == false)
    );
}

#[test]
fn retained_source_projection_reports_expiry_without_inventing_absence() {
    let (mut worker, fence) = prepared_rollover_worker();
    worker.lease = Some(2);
    feed_source(
        &mut worker,
        &fence,
        Provider::Kalshi,
        book_frame(3, "KXBTC15M-0", r#"[["0.36","1"]]"#, r#"[["0.64","1"]]"#).as_bytes(),
        1,
        None,
    )
    .unwrap();
    let view = worker.snapshot(1).unwrap();
    for (at, usable, reason) in [(1, true, None), (2, false, Some("stale_data"))] {
        let projected = view.wire_snapshot(at);
        for binding in projected["bindings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["market_id"] == "KXBTC15M-0")
        {
            assert_eq!(binding["usable"], usable);
            assert_eq!(binding["reason"], serde_json::json!(reason));
            assert!(!binding["value"].is_null());
        }
    }
    assert_eq!(
        worker.snapshot(2).unwrap().wire_snapshot(2)["bindings"][0]["reason"],
        "stale_data"
    );
    let (mut worker, _fence) = prepared_rollover_worker();
    let projected = worker.snapshot(0).unwrap().wire_snapshot(900000);
    for binding in projected["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["market_id"] == "KXBTC15M-0")
    {
        assert_eq!(binding["usable"], false);
        assert_eq!(binding["reason"], "market_closed");
    }
    assert_eq!(projected["markets"][0]["preparation"], "expired");
    assert!(projected["current_market"].is_null());
    use metten_engine::sources::{Scope, Stream, Unavailable};
    worker
        .state
        .invalidate(Scope::Stream(Stream::Book), Unavailable::Gap, 1, None)
        .unwrap();
    assert_eq!(
        worker.snapshot(1).unwrap().wire_snapshot(900000)["bindings"][0]["reason"],
        "gap"
    );
}

#[test]
fn sources_header_names_only_a_verified_current_market() {
    use metten_engine::sources::{Scope, SourceEvent, Stream, Unavailable};
    let (mut worker, fence) = prepared_rollover_worker();
    let projected = worker.snapshot(1).unwrap().wire_snapshot(1);
    assert_eq!(projected["current_market"], "KXBTC15M-0");
    assert_eq!(projected["markets"][0]["verified"], true);
    worker
        .state
        .invalidate(
            Scope::Stream(Stream::Metadata),
            Unavailable::MetadataChanged,
            1,
            None,
        )
        .unwrap();
    let projected = worker.snapshot(1).unwrap().wire_snapshot(1);
    assert_eq!(projected["markets"][0]["verified"], false);
    assert!(projected["current_market"].is_null());
    worker
        .state
        .apply(source_envelope(
            2,
            SourceEvent::Metadata {
                token: worker.state.token(Stream::Metadata),
                market_id: "KXBTC15M-0".into(),
                verified: true,
            },
        ))
        .unwrap();
    let published = worker.snapshot(2).unwrap();
    let projected = published.wire_snapshot(2);
    assert_eq!(projected["markets"][0]["verified"], true);
    assert_eq!(projected["current_market"], "KXBTC15M-0");
    for frame in [
        br#"{"type":"subscribed","msg":{"sid":4,"channel":"market_lifecycle_v2"}}"#.as_slice(),
        br#"{"type":"market_lifecycle_v2","sid":4,"seq":1,"msg":{"market_ticker":"KXBTC15M-0"}}"#,
    ] {
        feed_source(&mut worker, &fence, Provider::Kalshi, frame, 2, None).unwrap();
    }
    // The same published view, whose captured market is still verified, loses its metadata permission.
    assert!(!published.stream_current(Stream::Metadata));
    let projected = published.wire_snapshot(2);
    assert_eq!(projected["markets"][0]["verified"], false);
    assert!(projected["current_market"].is_null());
}

#[test]
fn production_and_isolated_sources_keep_distinct_native_environments() {
    let production = transport::Transport::new(Environment::Production).unwrap();
    let isolated = transport::Transport::isolated().unwrap();
    assert_eq!(production.environment().name(), "production");
    assert_eq!(
        production.environment().socket(),
        "wss://external-api-ws.kalshi.com/trade-api/ws/v2"
    );
    assert_eq!(isolated.environment().name(), "demo");
    assert_ne!(
        production.environment().account(),
        isolated.environment().account()
    );
    assert!(!production.is_isolated());
    assert!(isolated.is_isolated());
}

#[tokio::test]
async fn socket_intake_waits_within_age_limit_and_remains_cancelable() {
    use futures_util::SinkExt;
    for mode in ["drain", "stop", "expire"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            for n in 0..=RAW_SLOTS {
                socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        n.to_string().into(),
                    ))
                    .await
                    .unwrap();
            }
            futures_util::StreamExt::next(&mut socket).await;
        });
        let fence = Fence::new();
        let connection =
            transport::local_connection(address, Provider::Coinbase, fence.clone()).await;
        let transport = Arc::new(transport::Transport::new(Environment::Demo).unwrap());
        let (_control, mut controls) = mpsc::channel(1);
        let (tx, mut rx) = intake();
        let (signal, stop) = watch::channel(false);
        let source = metten_engine::sources::SourceState::display().unwrap();
        let tokens = SOURCE_STREAMS.map(|stream| source.token(stream));
        let running = transport.clone();
        let task = tokio::spawn(async move {
            running
                .serve(connection, &mut controls, tx, tokens, stop, None)
                .await
        });
        tokio::time::timeout(SHUTDOWN, async {
            while transport.counters(Provider::Coinbase).unwrap()[4] < (RAW_SLOTS + 1) as u64 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(
            !task.is_finished(),
            "full intake must wait within the existing age limit"
        );
        if mode == "drain" {
            for n in 0..=RAW_SLOTS {
                let item = tokio::time::timeout(SHUTDOWN, rx.recv())
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(item.bytes.as_slice(), n.to_string().as_bytes());
            }
        }
        if mode != "expire" {
            signal.send_replace(true);
        }
        let result = tokio::time::timeout(SHUTDOWN, task).await.unwrap().unwrap();
        assert_eq!(
            result,
            Err(if mode == "expire" {
                Failure::Deadline
            } else {
                Failure::Stopped
            })
        );
        assert!(!fence.current(fence.token()));
        peer.await.unwrap();
    }
}
