#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod credential;
#[cfg(target_os = "macos")]
mod paper;
#[cfg(target_os = "macos")]
mod provider_credential;
#[cfg(target_os = "macos")]
mod providers;

use metten_engine::{Action, Engine, Snapshot};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

struct Workspace {
    engine: Result<std::sync::Arc<Engine>, String>,
    #[cfg(target_os = "macos")]
    credentials: credential::Credentials,
    #[cfg(target_os = "macos")]
    providers: std::sync::Mutex<Option<providers::Coordinator>>,
    #[cfg(target_os = "macos")]
    provider_credentials: std::sync::Mutex<
        Option<provider_credential::Credentials<provider_credential::native::Keychain>>,
    >,
    #[cfg(target_os = "macos")]
    source_handle: std::sync::Mutex<Option<providers::SourceHandle>>,
    #[cfg(target_os = "macos")]
    source_transport: std::sync::Arc<providers::transport::Transport>,
    #[cfg(target_os = "macos")]
    source_revocation: std::sync::Mutex<providers::Revocation>,
    #[cfg(target_os = "macos")]
    source_verifier: std::sync::Mutex<Option<credential::SourceVerifier>>,
    provider_operation: std::sync::Arc<StrategyAdmission>,
    source_read: std::sync::Arc<StrategyAdmission>,
    source_allowed: bool,
    strategy_operation: std::sync::Arc<StrategyAdmission>,
    paper_operation: std::sync::Arc<StrategyAdmission>,
    paper_read: std::sync::Arc<StrategyAdmission>,
    #[cfg(target_os = "macos")]
    paper: std::sync::Mutex<paper::NativePaper>,
    #[cfg(target_os = "macos")]
    paper_stop: std::sync::Arc<AtomicBool>,
    quitting: AtomicBool,
    exit_ready: AtomicBool,
    quit_error: std::sync::Mutex<Option<String>>,
}

#[tauri::command]
fn snapshot(workspace: tauri::State<'_, Workspace>) -> Result<Snapshot, String> {
    let mut snapshot = workspace.engine.as_ref().map_err(Clone::clone)?.snapshot();
    if let Some(error) = workspace
        .quit_error
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
    {
        snapshot.recording_error = Some(error.clone());
    }
    Ok(snapshot)
}

#[tauri::command]
fn control(action: Action, workspace: tauri::State<'_, Workspace>) -> Result<(), String> {
    if workspace.quitting.load(Ordering::SeqCst) {
        return Err("Quitting — wait for the stopped state to be saved.".into());
    }
    workspace
        .engine
        .as_ref()
        .map_err(Clone::clone)?
        .request(action)
}

#[derive(Default)]
struct StrategyAdmission {
    busy: AtomicBool,
    completion: std::sync::Mutex<()>,
    idle: std::sync::Condvar,
}
struct StrategyGuard(std::sync::Arc<StrategyAdmission>);
impl Drop for StrategyGuard {
    fn drop(&mut self) {
        let _completion = self.0.completion.lock().unwrap_or_else(|e| e.into_inner());
        self.0.busy.store(false, Ordering::SeqCst);
        self.0.idle.notify_all();
    }
}
fn admit_strategy(
    operation: &std::sync::Arc<StrategyAdmission>,
    quitting: &AtomicBool,
) -> Result<StrategyGuard, metten_engine::StrategyFailure> {
    if quitting.load(Ordering::SeqCst) {
        return Err(metten_engine::StrategyFailure::new("quitting"));
    }
    operation
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| metten_engine::StrategyFailure::new("busy"))?;
    let guard = StrategyGuard(operation.clone());
    if quitting.load(Ordering::SeqCst) {
        return Err(metten_engine::StrategyFailure::new("quitting"));
    }
    Ok(guard)
}
fn decode_strategies(
    body: &tauri::ipc::InvokeBody,
) -> Result<metten_engine::StrategyRequest, metten_engine::StrategyFailure> {
    let tauri::ipc::InvokeBody::Json(body) = body else {
        return Err(metten_engine::StrategyFailure::new("invalid_request"));
    };
    metten_engine::decode_strategy_request(body)
}
fn wait_for_strategy(operation: &StrategyAdmission) {
    let mut completion = operation
        .completion
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    while operation.busy.load(Ordering::SeqCst) {
        completion = operation
            .idle
            .wait(completion)
            .unwrap_or_else(|e| e.into_inner());
    }
}
#[tauri::command]
async fn strategies(
    request: tauri::ipc::Request<'_>,
    app: tauri::AppHandle,
) -> Result<metten_engine::StrategyResponse, metten_engine::StrategyFailure> {
    let request = decode_strategies(request.body())?;
    let guard = {
        let workspace = app.state::<Workspace>();
        admit_strategy(&workspace.strategy_operation, &workspace.quitting)?
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let workspace = app.state::<Workspace>();
        if workspace.quitting.load(Ordering::SeqCst) {
            return Err(metten_engine::StrategyFailure::new("quitting"));
        }
        workspace
            .engine
            .as_ref()
            .map_err(|_| metten_engine::StrategyFailure::new("storage"))?
            .strategies_unless_quitting(request, &workspace.quitting)
    })
    .await
    .map_err(|_| metten_engine::StrategyFailure::new("storage"))?
}

// Decode manually: framework type errors must never quote submitted values.
fn decode_credential(body: &tauri::ipc::InvokeBody) -> Result<(&str, Option<&str>), &'static str> {
    const INVALID: &str = "Invalid credential request.";
    let tauri::ipc::InvokeBody::Json(body) = body else {
        return Err(INVALID);
    };
    let object = body.as_object().ok_or(INVALID)?;
    if object.keys().any(|k| k != "operation" && k != "value") {
        return Err(INVALID);
    }
    let operation = body
        .get("operation")
        .and_then(|v| v.as_str())
        .ok_or(INVALID)?;
    let value = match operation {
        "save" | "replace" => Some(body.get("value").and_then(|v| v.as_str()).ok_or(INVALID)?),
        "remove" | "check" if !object.contains_key("value") => None,
        _ => return Err(INVALID),
    };
    Ok((operation, value))
}

#[tauri::command]
async fn credential(
    request: tauri::ipc::Request<'_>,
    app: tauri::AppHandle,
) -> Result<&'static str, &'static str> {
    let (operation, value) = decode_credential(request.body())?;
    let operation = operation.to_owned();
    let value = value.map(str::to_owned);
    #[cfg(target_os = "macos")]
    return tauri::async_runtime::spawn_blocking(move || {
        let workspace = app.state::<Workspace>();
        let expected = if operation == "check" {
            workspace.credentials.expectation().map(Some)
        } else {
            Ok(None)
        };
        let _operation = workspace.credentials.begin_operation(&workspace.quitting)?;
        let expected = expected?;
        workspace
            .credentials
            .execute(&operation, value.as_deref(), expected.as_deref())
    })
    .await
    .map_err(|_| "Credential access unavailable — reopen the app.")?;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, operation, value);
        Err("Credential access requires the isolated Mac verification runner.")
    }
}

#[derive(Debug, serde::Serialize)]
struct SourceFailure {
    code: &'static str,
    message: &'static str,
}
impl SourceFailure {
    fn new(code: &'static str) -> Self {
        Self {
            code,
            message: match code {
                "busy" => "Source action already in progress. Wait and retry.",
                "quitting" | "still_stopping" => {
                    "Data intake is stopping. Wait before reconnecting."
                }
                "missing" => {
                    "Kalshi credentials are missing. Save the configured key before connecting."
                }
                "already_saved" => "A credential is already saved. Use Replace to change it.",
                "denied" => "Access was explicitly refused. Check the selected source permissions.",
                "invalid_credential" => "The key ID or private key is invalid or unsupported.",
                "capacity" => "Source data exceeds the bounded allowance.",
                "ambiguous" => {
                    "Credential change could not be confirmed. Inspect status before reconnecting."
                }
                "not_found" => "The saved strategy version is unavailable. Refresh saved versions.",
                "invalid_request" => "Invalid source request.",
                _ => "Source action unavailable. Inspect source status and retry explicitly.",
            },
        }
    }
}
#[cfg(target_os = "macos")]
impl From<providers::Failure> for SourceFailure {
    fn from(error: providers::Failure) -> Self {
        use providers::Failure::*;
        Self::new(match error {
            InvalidRequest => "invalid_request",
            InvalidCredential => "invalid_credential",
            Missing => "missing",
            AlreadySaved => "already_saved",
            Denied => "denied",
            Capacity => "capacity",
            Ambiguous => "ambiguous",
            StillStopping | Stopped => "still_stopping",
            _ => "unavailable",
        })
    }
}
const SOURCE_BYTES: usize = 256 * 1024;
struct CountBytes(usize);
impl std::io::Write for CountBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > SOURCE_BYTES.saturating_sub(self.0) {
            return Err(std::io::ErrorKind::OutOfMemory.into());
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn source_size(value: &impl serde::Serialize) -> Result<(), SourceFailure> {
    serde_json::to_writer(CountBytes(0), value).map_err(|_| SourceFailure::new("capacity"))
}
struct SourceResponse {
    value: serde_json::Value,
    #[cfg(target_os = "macos")]
    _charge: Option<tokio::sync::OwnedSemaphorePermit>,
}
impl serde::Serialize for SourceResponse {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}
#[derive(Debug, PartialEq)]
enum SourceRequest {
    Snapshot,
    Details(usize),
    Inspect(i64),
    Connect(usize),
    Disconnect(usize),
}
fn source_index(name: &str) -> Result<usize, SourceFailure> {
    ["kalshi", "brti", "coinbase", "kraken"]
        .iter()
        .position(|n| *n == name)
        .ok_or_else(|| SourceFailure::new("invalid_request"))
}
fn source_object<'a>(
    body: &'a tauri::ipc::InvokeBody,
    allowed: &[&str],
) -> Result<&'a serde_json::Map<String, serde_json::Value>, SourceFailure> {
    let invalid = || SourceFailure::new("invalid_request");
    let tauri::ipc::InvokeBody::Json(value) = body else {
        return Err(invalid());
    };
    let object = value.as_object().ok_or_else(invalid)?;
    if object.len() != allowed.len()
        || object.keys().any(|key| !allowed.contains(&key.as_str()))
        || object.values().any(|v| !v.is_string())
    {
        return Err(invalid());
    }
    source_size(value)?;
    Ok(object)
}
fn decode_sources(body: &tauri::ipc::InvokeBody) -> Result<SourceRequest, SourceFailure> {
    let tauri::ipc::InvokeBody::Json(value) = body else {
        return Err(SourceFailure::new("invalid_request"));
    };
    let operation = value
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let keys = match operation {
        "snapshot" => vec!["operation"],
        "inspect" => vec!["operation", "version_id"],
        "details" | "connect" | "disconnect" => vec!["operation", "source"],
        _ => return Err(SourceFailure::new("invalid_request")),
    };
    let object = source_object(body, &keys)?;
    Ok(match operation {
        "snapshot" => SourceRequest::Snapshot,
        "inspect" => {
            let id = object["version_id"].as_str().unwrap();
            if id.is_empty()
                || id.len() > 19
                || id.starts_with('0')
                || !id.bytes().all(|c| c.is_ascii_digit())
            {
                return Err(SourceFailure::new("invalid_request"));
            }
            SourceRequest::Inspect(
                id.parse::<i64>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| SourceFailure::new("invalid_request"))?,
            )
        }
        _ => {
            let index = source_index(object["source"].as_str().unwrap())?;
            match operation {
                "details" => SourceRequest::Details(index),
                "connect" => SourceRequest::Connect(index),
                _ => SourceRequest::Disconnect(index),
            }
        }
    })
}
#[cfg(target_os = "macos")]
struct ProviderOwner {
    app: tauri::AppHandle,
    coordinator: Option<providers::Coordinator>,
    credentials: Option<provider_credential::Credentials<provider_credential::native::Keychain>>,
    handle: Option<providers::SourceHandle>,
    handle_loaded: bool,
}
#[cfg(target_os = "macos")]
impl ProviderOwner {
    fn take(app: tauri::AppHandle) -> Result<Self, SourceFailure> {
        let workspace = app.state::<Workspace>();
        let mut owner = Self {
            app: app.clone(),
            coordinator: None,
            credentials: None,
            handle: None,
            handle_loaded: false,
        };
        owner.coordinator = workspace
            .providers
            .lock()
            .map_err(|_| SourceFailure::new("unavailable"))?
            .take();
        owner.credentials = workspace
            .provider_credentials
            .lock()
            .map_err(|_| SourceFailure::new("unavailable"))?
            .take();
        owner.handle = workspace
            .source_handle
            .lock()
            .map_err(|_| SourceFailure::new("unavailable"))?
            .as_ref()
            .cloned();
        owner.handle_loaded = true;
        if owner.coordinator.is_none() || owner.credentials.is_none() {
            return Err(SourceFailure::new("busy"));
        }
        Ok(owner)
    }
}
#[cfg(target_os = "macos")]
impl Drop for ProviderOwner {
    fn drop(&mut self) {
        let workspace = self.app.state::<Workspace>();
        if let Some(owner) = self.coordinator.take() {
            *workspace
                .providers
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(owner);
        }
        if let Some(credentials) = self.credentials.take() {
            *workspace
                .provider_credentials
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(credentials);
        }
        if self.handle_loaded {
            *workspace
                .source_handle
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = self.handle.take();
        }
    }
}

fn inspect_source_requirements(
    engine: &Engine,
    id: i64,
) -> Result<serde_json::Value, SourceFailure> {
    let result = engine
        .strategies(metten_engine::StrategyRequest::Detail { version_id: id })
        .map_err(|e| SourceFailure::new(e.code))?;
    let metten_engine::StrategyResponse::Detail { version } = result else {
        return Err(SourceFailure::new("unavailable"));
    };
    let rules = metten_engine::ValidatedStrategy::parse(
        version
            .json
            .as_deref()
            .ok_or_else(|| SourceFailure::new("unavailable"))?,
    )
    .map_err(|_| SourceFailure::new("unavailable"))?;
    let requirements:Vec<_>=rules.source_requirements().iter().map(|r|serde_json::json!({"input_id":r.input_id,"source":format!("{:?}",r.source).to_lowercase(),"instrument":r.instrument,"field":r.field,"unit":match r.unit {metten_engine::strategy::RuleUnit::ContractCents=>"contract_cents",metten_engine::strategy::RuleUnit::UsdPerBtc=>"usd_per_btc",metten_engine::strategy::RuleUnit::Percent=>"percent"},"side":r.side.map(|s|format!("{s:?}")),"startup_ms":r.startup_ms.to_string(),"implicit":r.implicit})).collect();
    let value = serde_json::json!({"kind":"requirements","version_id":id.to_string(),"requirements":requirements});
    source_size(&value)?;
    Ok(value)
}
#[tauri::command]
async fn sources(
    request: tauri::ipc::Request<'_>,
    app: tauri::AppHandle,
) -> Result<SourceResponse, SourceFailure> {
    let request = decode_sources(request.body())?;
    let reading = matches!(
        request,
        SourceRequest::Snapshot | SourceRequest::Details(_) | SourceRequest::Inspect(_)
    );
    // Before any side effect, including the early Disconnect revocation below.
    if !reading && !app.state::<Workspace>().source_allowed {
        return Err(SourceFailure::new("unavailable"));
    }
    #[cfg(target_os = "macos")]
    let requested_intent = if let SourceRequest::Connect(index) = &request {
        app.state::<Workspace>()
            .source_handle
            .lock()
            .map_err(|_| SourceFailure::new("unavailable"))?
            .as_ref()
            .map_or(0, |h| h.intent_revision(*index))
    } else {
        0
    };
    #[cfg(target_os = "macos")]
    if let SourceRequest::Disconnect(index) = &request {
        let handle = app
            .state::<Workspace>()
            .source_handle
            .lock()
            .map_err(|_| SourceFailure::new("unavailable"))?
            .as_ref()
            .cloned();
        if let Some(handle) = handle {
            handle.disable(*index)?;
        }
    }
    let guard = {
        let w = app.state::<Workspace>();
        admit_strategy(
            if reading {
                &w.source_read
            } else {
                &w.provider_operation
            },
            &w.quitting,
        )
        .map_err(|e| SourceFailure::new(e.code))?
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard=guard;
        let workspace=app.state::<Workspace>();
        if workspace.quitting.load(Ordering::SeqCst){return Err(SourceFailure::new("quitting"))}
        if let SourceRequest::Inspect(id)=request {
            let _inspection=admit_strategy(&workspace.strategy_operation,&workspace.quitting).map_err(|e|SourceFailure::new(e.code))?;
            let value=inspect_source_requirements(workspace.engine.as_ref().map_err(|_|SourceFailure::new("unavailable"))?,id)?;
            return Ok(SourceResponse{value,#[cfg(target_os="macos")]_charge:None});
        }
        #[cfg(target_os="macos")]
        {
            if reading {
                let (view,at,charge)={let handle=workspace.source_handle.lock().map_err(|_|SourceFailure::new("unavailable"))?;
                    let charge=workspace.source_transport.memory().reserve(SOURCE_BYTES*2)?;
                    (handle.as_ref().and_then(|h|h.latest.borrow().clone()),handle.as_ref().map_or(0,|h|h.elapsed_ms()),charge)};
                let mut value=match (request,view) {
                    (SourceRequest::Snapshot,Some(view))=>view.wire_snapshot(at),
                    (SourceRequest::Details(index),Some(view))=>view.wire_details(index,at)?,
                    (SourceRequest::Snapshot,None)=>serde_json::json!({"kind":"snapshot","owner":"0","revision":"0","observed_at_utc_ms":"0","current_market":null,"markets":[],"bindings":[],"sources":[{"source":"kalshi","connection":"disconnected","enabled":false},{"source":"brti","connection":"disconnected","enabled":false},{"source":"coinbase","connection":"disconnected","enabled":false},{"source":"kraken","connection":"disconnected","enabled":false}]}),
                    _=>return Err(SourceFailure::new("unavailable")),
                };
                if value["kind"] == "snapshot" {value["environment"]=serde_json::json!(workspace.source_transport.environment().name());}
                value["counters"]=serde_json::Value::Array([providers::Provider::Kalshi,providers::Provider::Coinbase,providers::Provider::Kraken].into_iter().map(|p|{
                    let counts=workspace.source_transport.counters(p)?;
                    Ok::<_,providers::Failure>(serde_json::json!({"provider":match p {providers::Provider::Kalshi=>"kalshi",providers::Provider::Coinbase=>"coinbase",providers::Provider::Kraken=>"kraken"},"rest_attempts":counts[0].to_string(),"handshake_attempts":counts[1].to_string(),"control_writes":counts[2].to_string(),"pong_flush_attempts":counts[3].to_string(),"data_messages":counts[4].to_string(),"slow_retry":workspace.source_transport.slow_retry(p)?}))
                }).collect::<Result<_,_>>()?);
                value["adapter_payload_bytes"]=serde_json::json!(workspace.source_transport.memory().used().to_string());
                source_size(&value)?;return Ok(SourceResponse{value,_charge:Some(charge)});
            }
            let mut owned=ProviderOwner::take(app.clone())?;
            let owner=owned.coordinator.as_mut().unwrap();
            let index=match request{SourceRequest::Connect(i)|SourceRequest::Disconnect(i)=>i,_=>return Err(SourceFailure::new("invalid_request"))};
            let provider=match index{0|1=>providers::Provider::Kalshi,2=>providers::Provider::Coinbase,_=>providers::Provider::Kraken};
            if matches!(request,SourceRequest::Connect(_)) && workspace.source_verifier.lock().map_err(|_|SourceFailure::new("unavailable"))?.as_ref().is_some_and(|v|v.failed()) {
                return Err(SourceFailure::new("still_stopping"));
            }
            if matches!(request,SourceRequest::Connect(_)) && owner.is_stopping() {
                tauri::async_runtime::block_on(owner.shutdown(tokio::time::Instant::now()+providers::SHUTDOWN))?;
                owner.recover(workspace.quitting.load(Ordering::SeqCst))?;
                owned.handle=None;
                *workspace.source_revocation.lock().map_err(|_|SourceFailure::new("unavailable"))?=owner.revocation();
            }
            if owned.handle.is_none(){
                owned.handle=Some(owner.start_sources(providers::SourceWorker::prepare_display(workspace.source_transport.memory().clone())?)?);
                *workspace.source_handle.lock().map_err(|_|SourceFailure::new("unavailable"))?=owned.handle.clone();
            }
            let handle=owned.handle.as_mut().unwrap();
            let intent=requested_intent;
            if matches!(request,SourceRequest::Disconnect(_)){
                handle.enable(index,false)?;
                let enabled=handle.enabled();
                if (index<2 && !enabled[0] && !enabled[1])||index>=2 {tauri::async_runtime::block_on(owner.stop_provider(provider,tokio::time::Instant::now()+providers::SHUTDOWN))?;}
            }else{
                if owner.is_stopping(){return Err(SourceFailure::new("still_stopping"))}
                let material=if index<2&&!owner.active(provider){
                    match owned.credentials.as_mut().unwrap().with_signer(workspace.source_transport.environment(),|id,key|Ok((id.to_owned(),key.clone()))) {
                        Ok(material)=>Some(material),
                        Err(error)=>{handle.enable_if_current(index,intent)?;handle.show_connection(provider,if error==providers::Failure::Missing {metten_engine::sources::Connection::MissingConfiguration}else{metten_engine::sources::Connection::Disconnected})?;return Err(error.into())}
                    }
                }else{None};
                if workspace.quitting.load(Ordering::SeqCst){return Err(SourceFailure::new("quitting"))}
                handle.enable_if_current(index,intent)?;
                if index==0 && workspace.source_transport.is_isolated() {handle.prepare_fixture_markets()?;}
                if !owner.active(provider){tauri::async_runtime::block_on(owner.stop_provider(provider,tokio::time::Instant::now()+providers::SHUTDOWN))?;owner.recover_provider(provider,false)?;owner.start_provider(provider,workspace.source_transport.clone(),handle,material)?;}
            }
            Ok(SourceResponse{value:serde_json::json!({"kind":"action","source":(["kalshi","brti","coinbase","kraken"][index]),"accepted":true}),_charge:None})
        }
        #[cfg(not(target_os="macos"))]
        Err(SourceFailure::new("unavailable"))
    }).await.map_err(|_|SourceFailure::new("unavailable"))?
}
type ProviderRequest = (String, Option<(String, String)>);
fn decode_provider_credential(
    body: &tauri::ipc::InvokeBody,
) -> Result<ProviderRequest, SourceFailure> {
    let tauri::ipc::InvokeBody::Json(value) = body else {
        return Err(SourceFailure::new("invalid_request"));
    };
    let operation = value
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let material = matches!(operation, "save" | "replace");
    let keys = if material {
        vec!["operation", "provider", "key_id", "private_key"]
    } else if matches!(operation, "status" | "remove") {
        vec!["operation", "provider"]
    } else {
        return Err(SourceFailure::new("invalid_request"));
    };
    let object = source_object(body, &keys)?;
    if object["provider"] != "kalshi" {
        return Err(SourceFailure::new("invalid_request"));
    }
    #[cfg(target_os = "macos")]
    if material
        && (!provider_credential::valid_id(object["key_id"].as_str().unwrap())
            || object["private_key"].as_str().unwrap().len() > 16384)
    {
        return Err(SourceFailure::new("invalid_credential"));
    }
    let operation = operation.to_owned();
    let material = material.then(|| {
        (
            object["key_id"].as_str().unwrap().to_owned(),
            object["private_key"].as_str().unwrap().to_owned(),
        )
    });
    Ok((operation, material))
}
#[tauri::command]
async fn provider_credential(
    request: tauri::ipc::Request<'_>,
    app: tauri::AppHandle,
) -> Result<SourceResponse, SourceFailure> {
    let (operation, material) = decode_provider_credential(request.body())?;
    if !app.state::<Workspace>().source_allowed {
        return Err(SourceFailure::new("unavailable"));
    }
    let guard = {
        let w = app.state::<Workspace>();
        admit_strategy(&w.provider_operation, &w.quitting)
            .map_err(|e| SourceFailure::new(e.code))?
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        #[cfg(target_os = "macos")]
        {
            let mut owned = ProviderOwner::take(app.clone())?;
            let workspace = app.state::<Workspace>();
            if workspace.quitting.load(Ordering::SeqCst) {
                return Err(SourceFailure::new("quitting"));
            }
            let credentials = owned.credentials.as_mut().unwrap();
            let value = if operation == "status" {
                match credentials.status(workspace.source_transport.environment())? {
                    provider_credential::Status::Missing => serde_json::json!({"kind":"missing"}),
                    provider_credential::Status::Saved { key_id, key_type } => {
                        serde_json::json!({"kind":"saved","key_id":key_id,"key_type":key_type})
                    }
                }
            } else {
                let mutation = match operation.as_str() {
                    "save" => provider_credential::Mutation::Save,
                    "replace" => provider_credential::Mutation::Replace,
                    _ => provider_credential::Mutation::Remove,
                };
                if let Some((_, key)) = &material {
                    let _ = providers::signing::Key::parse(key)?;
                }
                let outcome = credentials.change(
                    workspace.source_transport.environment(),
                    mutation,
                    material
                        .as_ref()
                        .map(|(id, key)| (id.as_str(), key.as_str())),
                    owned.coordinator.as_mut().unwrap(),
                );
                if operation != "save"
                    && let Some(handle) = &owned.handle
                {
                    handle.enable(0, false)?;
                    handle.enable(1, false)?;
                }
                outcome?;
                serde_json::json!({"kind":"credential_changed","operation":operation})
            };
            Ok(SourceResponse {
                value,
                _charge: None,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (app, operation, material);
            Err(SourceFailure::new("unavailable"))
        }
    })
    .await
    .map_err(|_| SourceFailure::new("unavailable"))?
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn paper(
    app: tauri::AppHandle,
    request: tauri::ipc::Request<'_>,
) -> Result<serde_json::Value, metten_engine::paper::Failure> {
    use metten_engine::paper::{Control, Failure, Request};
    let mut request = paper::decode(request.body())?;
    let guard = {
        let workspace = app.state::<Workspace>();
        if matches!(&request, Request::Resume { .. }) {
            paper::admit_resume(
                workspace
                    .engine
                    .as_ref()
                    .map_err(|_| Failure::new("storage"))?,
                &mut request,
            );
        }
        if let Request::Pause { run_id } | Request::StopAfterMarket { run_id } = &request {
            workspace
                .engine
                .as_ref()
                .map_err(|_| Failure::new("storage"))?
                .fence_paper_control(
                    *run_id,
                    if matches!(&request, Request::Pause { .. }) {
                        Control::Pause
                    } else {
                        Control::StopAfterMarket
                    },
                )?;
        }
        admit_strategy(
            if request.is_read() {
                &workspace.paper_read
            } else {
                &workspace.paper_operation
            },
            &workspace.quitting,
        )
        .map_err(|e| Failure::new(e.code))?
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let workspace = app.state::<Workspace>();
        if workspace.quitting.load(Ordering::SeqCst) {
            return Err(Failure::new("quitting"));
        }
        let engine = workspace
            .engine
            .as_ref()
            .map_err(|_| Failure::new("storage"))?
            .clone();
        let mut owner = workspace
            .paper
            .lock()
            .map_err(|_| Failure::new("unconfirmed"))?;
        paper::dispatch(engine, &mut owner, request)
    })
    .await
    .map_err(|_| Failure::new("unconfirmed"))?
}

fn wait_until_idle(operation: &StrategyAdmission, deadline: tokio::time::Instant) -> bool {
    let mut lock = operation
        .completion
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    while operation.busy.load(Ordering::SeqCst) {
        let Some(remaining) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
            return false;
        };
        let (next, timed) = operation
            .idle
            .wait_timeout(lock, remaining)
            .unwrap_or_else(|e| e.into_inner());
        lock = next;
        if timed.timed_out() {
            return false;
        }
    }
    true
}

fn begin_quit(app: &tauri::AppHandle) {
    let workspace = app.state::<Workspace>();
    if workspace.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    #[cfg(target_os = "macos")]
    let deadline = tokio::time::Instant::now() + providers::SHUTDOWN;
    if let Ok(engine) = &workspace.engine {
        engine.fence_paper_quit();
    }
    #[cfg(target_os = "macos")]
    workspace.paper_stop.store(true, Ordering::SeqCst);
    #[cfg(target_os = "macos")]
    {
        if let Some(verifier) = workspace
            .source_verifier
            .lock()
            .expect("source verifier poisoned")
            .as_ref()
        {
            verifier.request_stop();
        }
        workspace
            .source_revocation
            .lock()
            .expect("source revocation poisoned")
            .revoke();
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(target_os = "macos")]
        {
            let ready = {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let operation = app.state::<Workspace>();
                    wait_until_idle(&operation.provider_operation, deadline)
                        && wait_until_idle(&operation.paper_operation, deadline)
                        && wait_until_idle(&operation.paper_read, deadline)
                })
                .await
            };
            let mut source = app
                .state::<Workspace>()
                .providers
                .lock()
                .expect("provider owner poisoned")
                .take();
            let verifier_result = {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let mut verifier = app
                        .state::<Workspace>()
                        .source_verifier
                        .lock()
                        .expect("source verifier poisoned")
                        .take();
                    let complete = verifier.as_mut().is_none_or(|v| v.stop(deadline));
                    *app.state::<Workspace>()
                        .source_verifier
                        .lock()
                        .expect("source verifier poisoned") = verifier;
                    complete
                })
                .await
            };
            let paper_result = {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    paper::NativePaper::stop_owner(
                        &app.state::<Workspace>().paper,
                        deadline.into_std(),
                    )
                })
                .await
            };
            let result = if !matches!(ready, Ok(true))
                || !matches!(verifier_result, Ok(true))
                || !matches!(paper_result, Ok(true))
            {
                Err(providers::Failure::StillStopping)
            } else if let Some(source) = &mut source {
                source.shutdown(deadline).await
            } else {
                Err(providers::Failure::StillStopping)
            };
            let workspace = app.state::<Workspace>();
            // An operation that held the owner past the deadline restores it on drop; keep that.
            if let Some(source) = source {
                *workspace.providers.lock().expect("provider owner poisoned") = Some(source);
            }
            if result.is_err() {
                retain_quit_failure(
                    &app,
                    "Data intake stopped — shutdown could not be confirmed.".into(),
                );
                return;
            }
        }
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let workspace = app.state::<Workspace>();
            wait_for_strategy(&workspace.strategy_operation);
            #[cfg(target_os = "macos")]
            {
                #[cfg(target_os = "macos")]
                let _credential_operation = workspace
                    .credentials
                    .operation
                    .lock()
                    .expect("credential operation mutex poisoned");
                #[cfg(target_os = "macos")]
                let _provider_credentials = workspace
                    .provider_credentials
                    .lock()
                    .expect("provider credential owner poisoned");
            }
            let result = workspace
                .engine
                .as_ref()
                .map_or(Ok(()), |engine| engine.stop_and_save());
            match result {
                Ok(()) => {
                    workspace.exit_ready.store(true, Ordering::SeqCst);
                    app.exit(0);
                }
                Err(error) => {
                    retain_quit_failure(&app, error);
                }
            }
        })
        .await;
    });
}

/// A process refused the workspace lock must not manage the shared credential item or sources.
fn source_allowed(engine: &Result<std::sync::Arc<Engine>, String>, source_mode: bool) -> bool {
    engine.is_ok() && (source_mode || std::env::var_os("METTEN_VERIFY_FD").is_none())
}

fn retain_quit_failure(app: &tauri::AppHandle, error: String) {
    let workspace = app.state::<Workspace>();
    *workspace
        .quit_error
        .lock()
        .expect("quit error state poisoned") = Some(error);
    workspace.quitting.store(false, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn context() -> tauri::Context<tauri::Wry> {
    tauri::generate_context!()
}

fn main() {
    #[cfg(target_os = "macos")]
    if let Some(code) = paper::check_entry() {
        std::process::exit(code);
    }
    #[cfg(target_os = "macos")]
    if let Some(code) = credential::provider_check() {
        std::process::exit(code);
    }
    #[cfg(target_os = "macos")]
    let source_setup = match credential::source_setup() {
        Ok(setup) => setup,
        Err(_) => std::process::exit(1),
    };
    tauri::Builder::default()
        .menu(|app| {
            let menu = tauri::menu::Menu::default(app)?;
            let quit_text = tauri::menu::PredefinedMenuItem::quit(app, None)?.text()?;
            // macOS's predefined Quit terminates before Tauri can prevent exit.
            for item in menu.items()? {
                if let Some(submenu) = item.as_submenu() {
                    for (index, item) in submenu.items()?.iter().enumerate() {
                        if let Some(predefined) = item.as_predefined_menuitem() && predefined.text()? == quit_text {
                            submenu.remove_at(index)?;
                            submenu.insert(&tauri::menu::MenuItem::with_id(app, "quit-session", "Quit Metten", true, Some("CmdOrCtrl+Q"))?, index)?;
                        }
                    }
                }
            }
            Ok(menu)
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "quit-session" { begin_quit(app); }
        })
        .setup(move |app| {
            // Only the native process may select storage. Smoke tests use a fresh directory.
            let engine = (|| -> Result<Engine, String> {
                let directory = match std::env::var_os("METTEN_TEST_WORKSPACE") {
                    Some(path) => std::path::PathBuf::from(path),
                    None => app.path().app_data_dir().map_err(|e| e.to_string())?,
                };
                std::fs::create_dir_all(&directory).map_err(|e| format!("Cannot open workspace. Check disk space and folder permissions, then reopen. {e}"))?;
                Engine::open(directory.join("session.sqlite"))
            })().map(std::sync::Arc::new);
            let source_mode = source_setup.is_some();
            let source_allowed = source_allowed(&engine, source_mode);
            let (provider_store, channel) = if let Some((target,stream,session))=source_setup {
                (provider_credential::native::Keychain::isolated(target),Some((stream,session)))
            }else{(provider_credential::native::Keychain::local(),None)};
            let coordinator=providers::Coordinator::new();
            let revocation=coordinator.revocation();
            let paper = paper::NativePaper::default();
            let paper_stop = paper.stop_signal();
            app.manage(Workspace { engine, #[cfg(target_os = "macos")] credentials: if source_mode { credential::Credentials::disabled() } else { credential::Credentials::setup() }, #[cfg(target_os = "macos")] providers: std::sync::Mutex::new(Some(coordinator)), #[cfg(target_os = "macos")] provider_credentials: std::sync::Mutex::new(Some(provider_credential::Credentials::new(provider_store))), source_handle: std::sync::Mutex::new(None), source_transport: std::sync::Arc::new((if source_mode {providers::transport::Transport::isolated()}else{providers::transport::Transport::new(providers::Environment::Production)}).map_err(|_| "Cannot prepare source transport")?), source_revocation: std::sync::Mutex::new(revocation), source_verifier: std::sync::Mutex::new(None), provider_operation: std::sync::Arc::new(StrategyAdmission::default()), source_read: std::sync::Arc::new(StrategyAdmission::default()), source_allowed, strategy_operation: std::sync::Arc::new(StrategyAdmission::default()), paper_operation: std::sync::Arc::new(StrategyAdmission::default()), paper_read: std::sync::Arc::new(StrategyAdmission::default()), paper:std::sync::Mutex::new(paper), paper_stop, quitting: AtomicBool::new(false), exit_ready: AtomicBool::new(false), quit_error: std::sync::Mutex::new(None) });
            {
                let workspace = app.state::<Workspace>();
                *workspace.source_revocation.lock().expect("source revocation poisoned") = workspace.providers.lock().expect("provider owner poisoned").as_ref().expect("source owner").revocation();
            }
            if let Some((stream,session))=channel {
                let verifier=credential::SourceVerifier::start(stream,session,app.handle().clone());
                *app.state::<Workspace>().source_verifier.lock().expect("source verifier poisoned")=Some(verifier);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![snapshot, control, credential, strategies, sources, provider_credential, paper])
        .build(context())
        .expect("Could not open the Metten application")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !app.state::<Workspace>().exit_ready.load(Ordering::SeqCst) {
                    api.prevent_exit();
                    begin_quit(app);
                }
            }
            tauri::RunEvent::WindowEvent { event: tauri::WindowEvent::CloseRequested { api, .. }, .. } => {
                api.prevent_close();
                begin_quit(app);
            }
            _ => (),
        });
}

#[cfg(test)]
mod permissions_tests {
    #[cfg(target_os = "macos")]
    #[test]
    fn credential_decode_never_echoes_untrusted_input() {
        use serde_json::json;
        use tauri::ipc::InvokeBody;
        let marker = "metten-dummy-parser-only-marker";
        for body in [
            json!(marker),
            json!({"operation": marker}),
            json!({"operation":"save", "value": [marker]}),
            json!({"operation":"replace", "value": null}),
            json!({"operation":"remove", "value":marker}),
            json!({"operation":"check", "path":marker}),
            json!({"operation":"save", "value":marker, "extra":marker}),
        ] {
            assert_eq!(
                super::decode_credential(&InvokeBody::Json(body)),
                Err("Invalid credential request.")
            );
        }
        assert_eq!(
            super::decode_credential(&InvokeBody::Raw(marker.as_bytes().to_vec())),
            Err("Invalid credential request.")
        );
        for operation in ["save", "replace"] {
            let body = InvokeBody::Json(json!({"operation":operation,"value":marker}));
            assert_eq!(
                super::decode_credential(&body),
                Ok((operation, Some(marker)))
            );
        }
        for operation in ["remove", "check"] {
            let body = InvokeBody::Json(json!({"operation":operation}));
            assert_eq!(super::decode_credential(&body), Ok((operation, None)));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn direct_strategy_requests_are_bounded_sanitized_and_independent_of_credentials() {
        use serde_json::json;
        use tauri::ipc::InvokeBody;
        let marker = "private-marker";
        for body in [
            InvokeBody::Raw(marker.as_bytes().to_vec()),
            InvokeBody::Json(json!({"operation":marker})),
            InvokeBody::Json(json!({"operation":"preview","json":"x".repeat(262145)})),
            InvokeBody::Json(
                json!({"operation":"save","json":"{}","reviewed_json":"{}","extra":marker}),
            ),
            InvokeBody::Json(json!({"operation":"detail","version_id":9007199254740993u64})),
        ] {
            let error = super::decode_strategies(&body).unwrap_err();
            assert!(!serde_json::to_string(&error).unwrap().contains(marker));
        }
        assert!(
            super::decode_strategies(&InvokeBody::Json(
                json!({"operation":"detail","version_id":"9007199254740993"})
            ))
            .is_ok()
        );
    }

    #[test]
    fn strategy_admission_releases_on_success_error_cancel_and_panic() {
        use std::sync::{Arc, atomic::AtomicBool};
        let operation = Arc::new(super::StrategyAdmission::default());
        let quitting = AtomicBool::new(false);
        let held = super::admit_strategy(&operation, &quitting).unwrap();
        assert_eq!(
            super::admit_strategy(&operation, &quitting)
                .err()
                .unwrap()
                .code,
            "busy"
        );
        drop(held);
        let error: Result<(), ()> = {
            let _guard = super::admit_strategy(&operation, &quitting).unwrap();
            Err(())
        };
        assert!(error.is_err());
        assert!(
            std::panic::catch_unwind(|| {
                let _guard = super::admit_strategy(&operation, &quitting).unwrap();
                panic!("controlled failure");
            })
            .is_err()
        );
        // A rejected/canceled queued closure drops its owned guard without executing.
        let guard = super::admit_strategy(&operation, &quitting).unwrap();
        let task = move || {
            drop(guard);
        };
        drop(task);
        drop(super::admit_strategy(&operation, &quitting).unwrap());
        quitting.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            super::admit_strategy(&operation, &quitting)
                .err()
                .unwrap()
                .code,
            "quitting"
        );
    }

    #[test]
    fn quit_waits_for_admitted_strategy_before_stopping_the_session() {
        use std::sync::{Arc, atomic::AtomicBool};
        let operation = Arc::new(super::StrategyAdmission::default());
        let quitting = AtomicBool::new(false);
        let guard = super::admit_strategy(&operation, &quitting).unwrap();
        quitting.store(true, std::sync::atomic::Ordering::SeqCst);
        let (sent, received) = std::sync::mpsc::channel();
        let waiting = operation.clone();
        let task = std::thread::spawn(move || {
            super::wait_for_strategy(&waiting);
            sent.send(()).unwrap();
        });
        assert!(received.try_recv().is_err());
        drop(guard);
        received
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        task.join().unwrap();
        assert_eq!(
            super::admit_strategy(&operation, &quitting)
                .err()
                .unwrap()
                .code,
            "quitting"
        );
    }

    #[test]
    fn source_requests_are_bounded_exact_and_keep_large_ids_as_decimal_strings() {
        use serde_json::json;
        use tauri::ipc::InvokeBody;
        for value in [
            json!({"operation":"snapshot","source":"kalshi"}),
            json!({"operation":"details","source":"unknown"}),
            json!({"operation":"connect","source":"kalshi","url":"private-marker"}),
            json!({"operation":"inspect","version_id":9007199254740993u64}),
            json!({"operation":"inspect","version_id":"01"}),
            json!({"operation":"inspect","version_id":"9223372036854775808"}),
            json!({"operation":"inspect","version_id":"x".repeat(super::SOURCE_BYTES+1)}),
        ] {
            let error = super::decode_sources(&InvokeBody::Json(value)).unwrap_err();
            assert!(
                !serde_json::to_string(&error)
                    .unwrap()
                    .contains("private-marker")
            );
        }
        for id in ["1", "9007199254740993", "9223372036854775807"] {
            assert_eq!(
                super::decode_sources(&InvokeBody::Json(
                    json!({"operation":"inspect","version_id":id})
                ))
                .unwrap(),
                super::SourceRequest::Inspect(id.parse().unwrap())
            );
        }
        assert!(super::decode_sources(&InvokeBody::Raw(vec![])).is_err());
        assert!(super::source_size(&"\n".repeat(super::SOURCE_BYTES)).is_err());
    }
    #[test]
    fn provider_command_decoder_refuses_extra_targets_and_sanitizes_material() {
        use serde_json::json;
        use tauri::ipc::InvokeBody;
        for value in [
            json!({"operation":"status","provider":"coinbase","environment":"demo"}),
            json!({"operation":"status","provider":"kalshi","environment":"production"}),
            json!({"operation":"remove","provider":"kalshi","private_key":"private-marker"}),
            json!({"operation":"save","provider":"kalshi","key_id":"x","private_key":"x".repeat(16385)}),
            json!({"operation":"replace","provider":"kalshi","key_id":"bad\n","private_key":"private-marker"}),
        ] {
            let error = super::decode_provider_credential(&InvokeBody::Json(value)).unwrap_err();
            assert!(
                !serde_json::to_string(&error)
                    .unwrap()
                    .contains("private-marker")
            );
        }
        assert!(
            super::decode_provider_credential(&InvokeBody::Json(
                json!({"operation":"status","provider":"kalshi"})
            ))
            .is_ok()
        );
    }
    #[test]
    fn a_process_refused_the_workspace_lock_cannot_manage_sources_or_credentials() {
        let path = std::env::temp_dir().join(format!(
            "metten-source-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let open = |source_mode| {
            let engine =
                metten_engine::Engine::open(path.join("session.sqlite")).map(std::sync::Arc::new);
            let allowed = super::source_allowed(&engine, source_mode);
            (engine, allowed)
        };
        let (first, allowed) = open(true);
        assert!(first.is_ok() && allowed);
        let (second, allowed) = open(true);
        assert!(
            second.is_err(),
            "second process must be refused the workspace lock"
        );
        assert!(!allowed);
        drop(first);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn source_inspection_reads_the_saved_validated_version_without_changing_it() {
        let path = std::env::temp_dir().join(format!(
            "metten-source-inspect-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let engine = metten_engine::Engine::open(path.join("session.sqlite")).unwrap();
        let json = include_str!("../../examples/learning.json");
        let metten_engine::StrategyResponse::Saved { version } = engine
            .strategies(metten_engine::StrategyRequest::Save {
                json: json.into(),
                reviewed_json: json.into(),
            })
            .unwrap()
        else {
            panic!("expected saved version")
        };
        let inspected =
            super::inspect_source_requirements(&engine, version.id.parse().unwrap()).unwrap();
        assert_eq!(inspected["version_id"], version.id);
        assert!(
            inspected["requirements"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["source"] == "kalshi" && r["startup_ms"].is_string())
        );
        let metten_engine::StrategyResponse::Detail { version: after } = engine
            .strategies(metten_engine::StrategyRequest::Detail {
                version_id: version.id.parse().unwrap(),
            })
            .unwrap()
        else {
            panic!("expected detail")
        };
        assert_eq!(after.json.as_deref(), Some(json));
        engine.stop_and_save().unwrap();
        drop(engine);
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn app_commands_are_limited_to_the_local_main_window() {
        use tauri::ipc::Origin;
        let mut context = super::context();
        let authority = context.runtime_authority_mut();
        let remote = Origin::Remote {
            url: "https://example.invalid".parse().unwrap(),
        };
        for command in [
            "snapshot",
            "control",
            "credential",
            "strategies",
            "sources",
            "provider_credential",
            "paper",
        ] {
            assert!(
                authority
                    .resolve_access(command, "main", "main", &Origin::Local)
                    .is_some()
            );
            assert!(
                authority
                    .resolve_access(command, "other", "other", &Origin::Local)
                    .is_none()
            );
            assert!(
                authority
                    .resolve_access(command, "main", "main", &remote)
                    .is_none()
            );
        }
        assert!(
            authority
                .resolve_access("read_secret", "main", "main", &Origin::Local)
                .is_none()
        );
    }
}
