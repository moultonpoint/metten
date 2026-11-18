//! Dummy-only file-Keychain proof. No destination or identity comes from the screen.
use core_foundation::{
    array::CFArray,
    base::{CFType, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::CFDictionary,
    string::CFString,
};
use security_framework_sys::{base::SecKeychainRef, item::*, keychain::*};
use serde::Deserialize;
use std::{
    ffi::CString,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{fs::MetadataExt, net::UnixStream},
    },
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

// These public legacy lifecycle functions are absent from security-framework-sys.
unsafe extern "C" {
    fn SecKeychainGetPath(keychain: SecKeychainRef, length: *mut u32, path: *mut i8) -> i32;
    fn SecKeychainGetStatus(keychain: SecKeychainRef, status: *mut u32) -> i32;
}
use security_framework_sys::keychain_item::{
    SecItemAdd, SecItemCopyMatching, SecItemDelete, SecItemUpdate,
};
const UNAVAILABLE: &str = "Unavailable — reopen with the isolated dummy verification runner.";
const LOCKED: &str =
    "Locked — explicitly unlock the disposable Keychain in the runner, then retry.";
const CHANNEL: &str =
    "Verification unavailable — prepare one fresh expectation in the runner, then retry.";

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum VerificationMode {
    Dummy,
    Sources,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bootstrap {
    path: PathBuf,
    device: u64,
    inode: u64,
    password: String,
    session: String,
    #[serde(default)]
    mode: Option<VerificationMode>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expectation {
    session: String,
    sequence: u64,
    phase: String,
    expected: String,
}
#[derive(Default)]
struct Verifier {
    pending: Option<String>,
    failed: bool,
}
pub struct Credentials {
    target: Mutex<Option<Bootstrap>>,
    verifier: Arc<Mutex<Verifier>>,
    pub operation: Mutex<()>,
}
fn receive(
    stream: &mut UnixStream,
    bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), &'static str> {
    let mut offset = 0;
    while offset < bytes.len() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(CHANNEL)?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| CHANNEL)?;
        let n = stream.read(&mut bytes[offset..]).map_err(|_| CHANNEL)?;
        if n == 0 {
            return Err(CHANNEL);
        }
        offset += n;
    }
    Ok(())
}
fn frame(stream: &mut UnixStream) -> Result<Vec<u8>, &'static str> {
    bounded_frame(stream, 4096)
}
fn bounded_frame(stream: &mut UnixStream, ceiling: usize) -> Result<Vec<u8>, &'static str> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut length = [0; 4];
    receive(stream, &mut length, deadline)?;
    let length = u32::from_be_bytes(length) as usize;
    if !(1..=ceiling).contains(&length) {
        return Err(CHANNEL);
    }
    let mut bytes = vec![0; length];
    receive(stream, &mut bytes, deadline)?;
    Ok(bytes)
}
fn observe(mut stream: UnixStream, session: String, state: std::sync::Weak<Mutex<Verifier>>) {
    let mut sequence = 0;
    loop {
        let mut poll = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut poll, 1, 200) };
        let Some(state) = state.upgrade() else {
            return;
        };
        if ready == 0 {
            continue;
        }
        let received = if ready > 0 {
            frame(&mut stream)
                .ok()
                .and_then(|b| serde_json::from_slice::<Expectation>(&b).ok())
        } else {
            None
        };
        let Ok(mut state) = state.lock() else {
            return;
        };
        let valid = received.filter(|e| {
            !state.failed
                && state.pending.is_none()
                && e.session == session
                && e.sequence == sequence + 1
                && e.phase == "local-use"
                && dummy(&e.expected)
        });
        match valid {
            Some(e) => {
                sequence += 1;
                state.pending = Some(e.expected);
                // Acknowledge receipt before the runner may press the UI command.
                if stream.write_all(&[1]).is_err() {
                    state.failed = true;
                    state.pending = None;
                    return;
                }
            }
            None => {
                state.failed = true;
                state.pending = None;
                let _ = stream.write_all(&[0]);
                return;
            }
        }
    }
}
fn map_status(status: i32) -> Result<(), &'static str> {
    match status {
        0 => Ok(()),
        -25299 => Err("Already saved — use Replace to change the dummy credential."),
        -25300 => Err("Missing — save a dummy credential first."),
        -128 => Err("Canceled — access was canceled; retry when ready."),
        -25293 => Err("Authentication failed — check the disposable Keychain setup in the runner."),
        -25308 => {
            Err("Denied — macOS requires access; check the disposable Keychain prompt and retry.")
        }
        -25243 => Err("Denied — macOS refused access to the disposable Keychain."),
        _ => Err(UNAVAILABLE),
    }
}
fn dummy(value: &str) -> bool {
    value.starts_with("metten-dummy-")
        && (20..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
impl Bootstrap {
    fn validate(&self) -> Result<(), &'static str> {
        let parent = self.path.parent().ok_or(UNAVAILABLE)?;
        let name = parent
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(UNAVAILABLE)?;
        let path = self.path.to_str().ok_or(UNAVAILABLE)?;
        if path == "/Library/Keychains/System.keychain"
            || path.contains("/login.keychain")
            || !name.starts_with("metten-credential-")
            || self.path.file_name().and_then(|s| s.to_str()) != Some("dummy.keychain-db")
            || self.path.canonicalize().ok().as_ref() != Some(&self.path)
        {
            return Err(UNAVAILABLE);
        }
        let dir = std::fs::symlink_metadata(parent).map_err(|_| UNAVAILABLE)?;
        let file = std::fs::symlink_metadata(&self.path).map_err(|_| UNAVAILABLE)?;
        // Canonical equality rejects path aliases; identity rejects replaced files.
        if !dir.is_dir()
            || dir.mode() & 0o777 != 0o700
            || dir.uid() != unsafe { libc::geteuid() }
            || !file.is_file()
            || file.uid() != dir.uid()
            || file.dev() != self.device
            || file.ino() != self.inode
            || file.nlink() != 1
        {
            return Err(UNAVAILABLE);
        }
        Ok(())
    }
    pub(crate) fn record_native_write(&mut self) -> Result<(), &'static str> {
        // Security atomically replaces file-Keychains on successful writes.
        // Accept a new file identity only immediately after our own successful API call.
        let file = std::fs::symlink_metadata(&self.path).map_err(|_| UNAVAILABLE)?;
        if !file.is_file()
            || file.uid() != unsafe { libc::geteuid() }
            || file.nlink() != 1
            || file.dev() != self.device
        {
            return Err(UNAVAILABLE);
        }
        self.inode = file.ino();
        self.validate()
    }
    fn activate(&mut self) -> Result<(), &'static str> {
        if !(32..=128).contains(&self.password.len()) || self.session.len() != 32 {
            return Err(UNAVAILABLE);
        }
        let keychain = self.open()?;
        unsafe {
            map_status(SecKeychainUnlock(
                keychain.as_CFTypeRef() as _,
                self.password.len() as u32,
                self.password.as_ptr().cast(),
                1,
            ))?;
        }
        unlocked(&keychain)?;
        self.password = String::new();
        Ok(())
    }
    pub(crate) fn open(&self) -> Result<CFType, &'static str> {
        self.validate()?;
        let path =
            CString::new(self.path.as_os_str().as_encoded_bytes()).map_err(|_| UNAVAILABLE)?;
        let mut raw = std::ptr::null_mut();
        unsafe {
            map_status(SecKeychainOpen(path.as_ptr(), &mut raw))?;
            if raw.is_null() {
                return Err(UNAVAILABLE);
            }
            let owned = CFType::wrap_under_create_rule(raw.cast());
            let mut buffer = [0u8; 4096];
            let mut length = buffer.len() as u32;
            map_status(SecKeychainGetPath(
                raw,
                &mut length,
                buffer.as_mut_ptr().cast(),
            ))?;
            if length as usize > buffer.len()
                || buffer[..length as usize]
                    .strip_suffix(&[0])
                    .unwrap_or(&buffer[..length as usize])
                    != self.path.as_os_str().as_encoded_bytes()
            {
                return Err(UNAVAILABLE);
            }
            self.validate()?;
            Ok(owned)
        }
    }
}
fn read_bootstrap(sources: bool) -> Result<(Bootstrap, UnixStream), &'static str> {
    if std::env::var("METTEN_VERIFY_FD").ok().as_deref() != Some("3") {
        return Err(UNAVAILABLE);
    }
    let mut stream = unsafe { UnixStream::from_raw_fd(3) };
    stream.peer_addr().map_err(|_| UNAVAILABLE)?;
    let mut target: Bootstrap =
        serde_json::from_slice(&frame(&mut stream)?).map_err(|_| UNAVAILABLE)?;
    if (target.mode == Some(VerificationMode::Sources)) != sources {
        return Err(UNAVAILABLE);
    }
    target.activate()?;
    Ok((target, stream))
}
pub(crate) fn source_setup() -> Result<Option<(Bootstrap, UnixStream, String)>, &'static str> {
    if std::env::var("METTEN_SOURCES_CHECK").ok().as_deref() != Some("1") {
        return Ok(None);
    }
    let (target, stream) = read_bootstrap(true)?;
    let session = target.session.clone();
    Ok(Some((target, stream, session)))
}
impl Credentials {
    pub fn disabled() -> Self {
        Self {
            target: Mutex::new(None),
            verifier: Arc::new(Mutex::new(Verifier::default())),
            operation: Mutex::new(()),
        }
    }

    pub fn begin_operation<'a>(
        &'a self,
        quitting: &std::sync::atomic::AtomicBool,
    ) -> Result<std::sync::MutexGuard<'a, ()>, &'static str> {
        let guard = self.operation.lock().map_err(|_| UNAVAILABLE)?;
        if quitting.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("Quitting — credential action refused.");
        }
        Ok(guard)
    }
    pub fn setup() -> Self {
        let setup = read_bootstrap(false);
        let verifier = Arc::new(Mutex::new(Verifier::default()));
        let target = match setup {
            Ok((target, stream)) => {
                let session = target.session.clone();
                let weak = Arc::downgrade(&verifier);
                std::thread::spawn(move || observe(stream, session, weak));
                Some(target)
            }
            Err(_) => None,
        };
        Self {
            target: Mutex::new(target),
            verifier,
            operation: Mutex::new(()),
        }
    }
    // Receipt and framing finish before the UI command; this never waits on input.
    pub fn expectation(&self) -> Result<String, &'static str> {
        let mut v = self.verifier.lock().map_err(|_| CHANNEL)?;
        if v.failed {
            return Err(CHANNEL);
        }
        v.pending.take().ok_or(CHANNEL)
    }
    pub fn execute(
        &self,
        operation: &str,
        value: Option<&str>,
        expected: Option<&str>,
    ) -> Result<&'static str, &'static str> {
        let mut target_guard = self.target.lock().map_err(|_| UNAVAILABLE)?;
        let target = target_guard.as_mut().ok_or(UNAVAILABLE)?;
        let keychain = target.open()?;
        unlocked(&keychain)?;
        unsafe {
            let mut pairs = vec![
                (
                    CFString::wrap_under_get_rule(kSecClass).as_CFType(),
                    CFString::wrap_under_get_rule(kSecClassGenericPassword).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrService).as_CFType(),
                    CFString::new("com.metten.dummy-connection").as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrAccount).as_CFType(),
                    CFString::new("dummy").as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrSynchronizable).as_CFType(),
                    CFBoolean::false_value().as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecUseDataProtectionKeychain).as_CFType(),
                    CFBoolean::false_value().as_CFType(),
                ),
            ];
            if operation == "save" {
                let value = value
                    .filter(|v| dummy(v))
                    .ok_or("Invalid input — enter a bounded dummy verification value.")?;
                pairs.push((
                    CFString::wrap_under_get_rule(kSecUseKeychain).as_CFType(),
                    keychain.clone(),
                ));
                pairs.push((
                    CFString::wrap_under_get_rule(kSecValueData).as_CFType(),
                    CFData::from_buffer(value.as_bytes()).as_CFType(),
                ));
                map_status(SecItemAdd(
                    CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef(),
                    std::ptr::null_mut(),
                ))?;
                target.record_native_write()?;
                return Ok(
                    "Saved — local readability has not been checked. No exchange is connected.",
                );
            }
            pairs.push((
                CFString::wrap_under_get_rule(kSecMatchSearchList).as_CFType(),
                CFArray::from_CFTypes(&[keychain]).as_CFType(),
            ));
            match operation {
                "replace" => {
                    let value = value
                        .filter(|v| dummy(v))
                        .ok_or("Invalid input — enter a bounded dummy verification value.")?;
                    let update = CFDictionary::from_CFType_pairs(&[(
                        CFString::wrap_under_get_rule(kSecValueData),
                        CFData::from_buffer(value.as_bytes()),
                    )]);
                    map_status(SecItemUpdate(
                        CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef(),
                        update.as_concrete_TypeRef(),
                    ))?;
                    target.record_native_write()?;
                    Ok("Replaced — check local credential use. No exchange is connected.")
                }
                "remove" => {
                    map_status(SecItemDelete(
                        CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef(),
                    ))?;
                    target.record_native_write()?;
                    Ok("Removed — no exchange is connected.")
                }
                "check" => {
                    pairs.push((
                        CFString::wrap_under_get_rule(kSecReturnData).as_CFType(),
                        CFBoolean::true_value().as_CFType(),
                    ));
                    let mut result = std::ptr::null();
                    map_status(SecItemCopyMatching(
                        CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef(),
                        &mut result,
                    ))?;
                    let data = CFType::wrap_under_create_rule(result);
                    let data = data.downcast::<CFData>().ok_or(UNAVAILABLE)?;
                    // The observer sees only bytes returned by this actual trusted-use read.
                    if expected.is_some_and(|e| data.bytes() == e.as_bytes()) {
                        Ok(
                            "Readable — dummy local-use comparison passed. No exchange is connected.",
                        )
                    } else {
                        Err("Mismatch — local-use comparison failed; the saved item is unchanged.")
                    }
                }
                _ => Err("Invalid request — choose a supported credential action."),
            }
        }
    }
}
pub(crate) fn unlocked(keychain: &CFType) -> Result<(), &'static str> {
    let mut status = 0;
    unsafe {
        map_status(SecKeychainGetStatus(
            keychain.as_CFTypeRef() as _,
            &mut status,
        ))?;
    }
    if status & 1 == 0 { Err(LOCKED) } else { Ok(()) }
}
// The step-3 packaged check consumes the same private descriptor and validated
// disposable target, before ordinary startup; it never creates a renderer command.
pub(crate) fn provider_check() -> Option<i32> {
    if std::env::var("METTEN_PROVIDER_CHECK").ok().as_deref() != Some("1") {
        return None;
    }
    let result = (|| -> Result<(), &'static str> {
        let (target, mut stream) = read_bootstrap(true)?;
        crate::provider_credential::verify_packaged(target).map_err(|_| UNAVAILABLE)?;
        stream.write_all(&[1]).map_err(|_| UNAVAILABLE)?;
        Ok(())
    })();
    Some(if result.is_ok() { 0 } else { 1 })
}
#[cfg(test)]
impl Bootstrap {
    pub(crate) fn fixture(path: PathBuf) -> Self {
        let path = path.canonicalize().unwrap();
        let m = std::fs::metadata(&path).unwrap();
        Self {
            path,
            device: m.dev(),
            inode: m.ino(),
            password: String::new(),
            session: String::new(),
            mode: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn isolated_state() -> Credentials {
        Credentials {
            target: Mutex::new(None),
            verifier: Arc::new(Mutex::new(Verifier::default())),
            operation: Mutex::new(()),
        }
    }
    #[test]
    fn bootstrap_modes_are_finite_strict_and_default_to_dummy() {
        let fields = r#""path":"/invalid","device":1,"inode":1,"password":"controlled","session":"controlled""#;
        let parse =
            |extra: &str| serde_json::from_str::<Bootstrap>(&format!("{{{fields}{extra}}}"));
        assert!(parse("").unwrap().mode.is_none());
        assert!(parse(",\"mode\":\"dummy\"").unwrap().mode == Some(VerificationMode::Dummy));
        assert!(parse(",\"mode\":\"sources\"").unwrap().mode == Some(VerificationMode::Sources));
        for bad in [
            ",\"mode\":\"unknown\"",
            ",\"mode\":\"sources\",\"mode\":\"dummy\"",
            ",\"extra\":1",
        ] {
            assert!(parse(bad).is_err());
        }
    }
    #[test]
    fn queued_actions_refuse_after_quit_and_quit_waits_for_current_work() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            mpsc,
        };
        let credentials = isolated_state();
        let quitting = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let active = credentials.begin_operation(&quitting).unwrap();
            let (started, waiting) = mpsc::channel();
            let mut queued = Vec::new();
            for _ in ["save", "replace", "remove", "check"] {
                let started = started.clone();
                let credentials = &credentials;
                let quitting = &quitting;
                queued.push(scope.spawn(move || {
                    started.send(()).unwrap();
                    assert_eq!(
                        credentials.begin_operation(quitting).err(),
                        Some("Quitting — credential action refused.")
                    );
                }));
            }
            for _ in 0..4 {
                waiting.recv().unwrap();
            }
            quitting.store(true, Ordering::SeqCst);
            let (finished, stopped) = mpsc::channel();
            let credentials = &credentials;
            let quit = scope.spawn(move || {
                let _pending = credentials.operation.lock().unwrap();
                finished.send(()).unwrap();
            });
            assert!(stopped.try_recv().is_err());
            assert!(credentials.operation.try_lock().is_err());
            drop(active);
            for command in queued {
                command.join().unwrap();
            }
            quit.join().unwrap();
            stopped.recv().unwrap();
        });
        assert!(credentials.begin_operation(&quitting).is_err());
    }
    #[test]
    fn target_rejects_unexplained_replacement_aliases_and_shared_files() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "metten-credential-identity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("dummy.keychain-db");
        std::fs::write(&path, []).unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        let target = Bootstrap {
            path: path.clone(),
            device: metadata.dev(),
            inode: metadata.ino(),
            password: String::new(),
            session: String::new(),
            mode: None,
        };
        assert!(target.validate().is_ok());
        let alias = directory.join("alias");
        std::fs::hard_link(&path, &alias).unwrap();
        assert!(target.validate().is_err());
        std::fs::remove_file(&alias).unwrap();
        std::fs::rename(&path, &alias).unwrap();
        std::fs::write(&path, []).unwrap();
        assert!(target.validate().is_err());
        std::fs::remove_file(&path).unwrap();
        symlink(&alias, &path).unwrap();
        assert!(target.validate().is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&alias, &path).unwrap();
        assert!(target.validate().is_ok());
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(target.validate().is_err());
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&directory).unwrap();
    }
    #[test]
    fn input_and_failures_are_bounded() {
        assert!(dummy("metten-dummy-12345678"));
        assert!(!dummy("real credential"));
        assert!(!dummy(&"metten-dummy-".repeat(20)));
        for code in [-128, -25293, -25308, -25243, -25300, -25299, -1] {
            assert!(map_status(code).is_err());
        }
        assert!(map_status(0).is_ok());
    }
    #[test]
    fn channel_frames_are_bounded_and_closed_input_fails() {
        use std::io::Write;
        let (mut a, mut b) = UnixStream::pair().unwrap();
        a.write_all(&4097u32.to_be_bytes()).unwrap();
        assert!(frame(&mut b).is_err());
        drop(a);
        assert!(frame(&mut b).is_err());
    }
    #[test]
    fn expectation_is_received_once_and_duplicate_is_rejected() {
        use std::io::{Read, Write};
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        sender
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let state = Arc::new(Mutex::new(Verifier::default()));
        let weak = Arc::downgrade(&state);
        let thread = std::thread::spawn(move || observe(receiver, "session".into(), weak));
        let send = |sender: &mut UnixStream, sequence: u64| {
            let bytes = serde_json::to_vec(&serde_json::json!({"session":"session", "sequence":sequence, "phase":"local-use", "expected":"metten-dummy-12345678"})).unwrap();
            sender
                .write_all(&(bytes.len() as u32).to_be_bytes())
                .unwrap();
            sender.write_all(&bytes).unwrap();
            let mut ack = [255];
            sender.read_exact(&mut ack).unwrap();
            ack[0]
        };
        assert_eq!(send(&mut sender, 1), 1);
        assert!(state.lock().unwrap().pending.take().is_some());
        assert!(state.lock().unwrap().pending.take().is_none());
        assert_eq!(send(&mut sender, 1), 0);
        assert!(state.lock().unwrap().failed);
        thread.join().unwrap();
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFrame<'a> {
    session: &'a str,
    sequence: &'a str,
    kind: &'a str,
    provider: &'a str,
    #[serde(borrow)]
    payload: std::borrow::Cow<'a, str>,
}
fn source_frame<'a>(
    bytes: &'a [u8],
    session: &str,
    expected: u64,
) -> Result<(crate::providers::Provider, std::borrow::Cow<'a, str>), &'static str> {
    use crate::providers::Provider;
    if bytes.len() > 256 * 1024 {
        return Err(CHANNEL);
    }
    let frame: SourceFrame<'a> = serde_json::from_slice(bytes).map_err(|_| CHANNEL)?;
    if frame.session != session
        || frame.kind != "provider_frame"
        || frame.sequence != expected.to_string()
        || expected == 0
        || frame.payload.len() > 128 * 1024
    {
        return Err(CHANNEL);
    }
    let provider = match frame.provider {
        "kalshi" => Provider::Kalshi,
        "coinbase" => Provider::Coinbase,
        "kraken" => Provider::Kraken,
        _ => return Err(CHANNEL),
    };
    Ok((provider, frame.payload))
}
fn observe_sources(
    stream: &mut UnixStream,
    session: &str,
    stopped: &std::sync::atomic::AtomicBool,
    memory: &crate::providers::memory::Memory,
    mut dispatch: impl FnMut(crate::providers::Provider, &[u8]) -> Result<(), crate::providers::Failure>,
) -> Result<(), &'static str> {
    use std::sync::atomic::Ordering;
    let mut sequence = 1u64;
    while !stopped.load(Ordering::SeqCst) {
        let mut poll = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut poll, 1, 50) };
        if ready < 0 {
            return Err(CHANNEL);
        }
        if ready == 0 {
            continue;
        }
        let _charge = memory.reserve(640 * 1024).map_err(|_| CHANNEL)?;
        let bytes = bounded_frame(stream, 256 * 1024)?;
        let (provider, payload) = source_frame(&bytes, session, sequence)?;
        if stopped.load(Ordering::SeqCst) {
            return Err(CHANNEL);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        dispatch(provider, payload.as_bytes()).map_err(|_| CHANNEL)?;
        if stopped.load(Ordering::SeqCst) {
            return Err(CHANNEL);
        }
        stream
            .set_write_timeout(Some(
                deadline
                    .checked_duration_since(Instant::now())
                    .ok_or(CHANNEL)?,
            ))
            .map_err(|_| CHANNEL)?;
        stream.write_all(&[1]).map_err(|_| CHANNEL)?;
        sequence = sequence.checked_add(1).ok_or(CHANNEL)?;
    }
    Ok(())
}
struct SourceReceiverLifetime {
    revocation: crate::providers::Revocation,
    stop: Arc<std::sync::atomic::AtomicBool>,
    failed: Arc<std::sync::atomic::AtomicBool>,
}
impl Drop for SourceReceiverLifetime {
    fn drop(&mut self) {
        if !self.stop.load(std::sync::atomic::Ordering::SeqCst) {
            self.failed.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        self.revocation.revoke();
    }
}
pub(crate) struct SourceVerifier {
    stop: Arc<std::sync::atomic::AtomicBool>,
    failed: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl SourceVerifier {
    pub fn start(mut stream: UnixStream, session: String, app: tauri::AppHandle) -> Self {
        use std::sync::atomic::{AtomicBool, Ordering};
        use tauri::Manager;
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let failed = Arc::new(AtomicBool::new(false));
        let failure = failed.clone();
        let memory = app
            .state::<crate::Workspace>()
            .source_transport
            .memory()
            .clone();
        let lifetime = SourceReceiverLifetime {
            revocation: app
                .state::<crate::Workspace>()
                .source_revocation
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            stop: stop.clone(),
            failed: failed.clone(),
        };
        let thread = std::thread::spawn(move || {
            let _lifetime = lifetime;
            let result = observe_sources(
                &mut stream,
                &session,
                &stopped,
                &memory,
                |provider, payload| {
                    let handle = app
                        .state::<crate::Workspace>()
                        .source_handle
                        .lock()
                        .map_err(|_| crate::providers::Failure::Unavailable)?
                        .as_ref()
                        .cloned()
                        .ok_or(crate::providers::Failure::Stopped)?;
                    handle.dispatch_fixture(provider, payload)
                },
            );
            if result.is_err() {
                failure.store(true, Ordering::SeqCst);
                let _ = stream.set_write_timeout(Some(Duration::from_millis(10)));
                let _ = stream.write_all(&[0]);
            }
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });
        Self {
            stop,
            failed,
            thread: Some(thread),
        }
    }
    pub fn failed(&self) -> bool {
        self.failed.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub fn request_stop(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn stop(&mut self, deadline: tokio::time::Instant) -> bool {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        while self.thread.as_ref().is_some_and(|t| !t.is_finished())
            && tokio::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        if self.thread.as_ref().is_some_and(|t| !t.is_finished()) {
            return false;
        }
        self.thread
            .take()
            .is_none_or(|thread| thread.join().is_ok())
    }
}

#[cfg(test)]
mod source_protocol_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn source_envelopes_are_exact_sequenced_and_mode_specific() {
        let session = "0123456789abcdef0123456789abcdef";
        let valid = json!({"session":session,"sequence":"1","kind":"provider_frame","provider":"kraken","payload":"{\"price\":68000.123456}"});
        let bytes = serde_json::to_vec(&valid).unwrap();
        let (provider, payload) = source_frame(&bytes, session, 1).unwrap();
        assert_eq!(provider, crate::providers::Provider::Kraken);
        assert_eq!(payload, "{\"price\":68000.123456}");
        assert!(serde_json::from_slice::<Expectation>(&bytes).is_err());
        assert!(source_frame(&bytes, "wrong", 1).is_err());
        assert!(source_frame(&bytes, session, 2).is_err());
        for (key, value) in [
            ("sequence", json!(1)),
            ("sequence", json!("01")),
            ("sequence", json!("0")),
            ("sequence", json!("18446744073709551616")),
            ("kind", json!("expectation")),
            ("provider", json!("brti")),
            ("extra", json!(true)),
            ("payload", json!("x".repeat(128 * 1024 + 1))),
        ] {
            let mut bad = valid.clone();
            bad[key] = value;
            assert!(
                source_frame(&serde_json::to_vec(&bad).unwrap(), session, 1).is_err(),
                "accepted {key}"
            );
        }
        let mut maximum = valid.clone();
        maximum["sequence"] = json!(u64::MAX.to_string());
        assert!(source_frame(&serde_json::to_vec(&maximum).unwrap(), session, u64::MAX).is_ok());
        assert!(source_frame(&vec![b' '; 256 * 1024 + 1], session, 1).is_err());
        assert!(
            source_frame(
                br#"{"phase":"local-use","expected":"dummy","sequence":1}"#,
                session,
                1
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod source_receiver_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    fn packet(sequence: &str) -> Vec<u8> {
        let bytes=serde_json::to_vec(&serde_json::json!({"session":"0123456789abcdef0123456789abcdef","sequence":sequence,"kind":"provider_frame","provider":"coinbase","payload":"not json"})).unwrap();
        let mut framed = (bytes.len() as u32).to_be_bytes().to_vec();
        framed.extend(bytes);
        framed
    }
    #[test]
    fn acknowledgment_waits_for_dispatch_and_duplicate_sequence_fails_without_redispatch() {
        let (mut peer, mut receiver) = UnixStream::pair().unwrap();
        let (entered, entry) = std::sync::mpsc::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let memory = crate::providers::memory::Memory::default();
        let owned = memory.clone();
        let thread = std::thread::spawn(move || {
            observe_sources(
                &mut receiver,
                "0123456789abcdef0123456789abcdef",
                &AtomicBool::new(false),
                &owned,
                |_, _| {
                    entered.send(()).unwrap();
                    gate.recv_timeout(Duration::from_secs(1)).unwrap();
                    Ok(())
                },
            )
        });
        peer.write_all(&packet("1")).unwrap();
        entry.recv_timeout(Duration::from_secs(1)).unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut ack = [0];
        assert!(
            peer.read_exact(&mut ack).is_err(),
            "acknowledged before publication completed"
        );
        release.send(()).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        peer.read_exact(&mut ack).unwrap();
        assert_eq!(ack, [1]);
        peer.write_all(&packet("1")).unwrap();
        assert!(thread.join().unwrap().is_err());
        assert!(entry.try_recv().is_err());
        assert_eq!(memory.used(), 0);
    }
    #[test]
    fn quit_or_eof_during_dispatch_never_acknowledges_success() {
        for quit in [false, true] {
            let (mut peer, mut receiver) = UnixStream::pair().unwrap();
            let (entered, entry) = std::sync::mpsc::channel();
            let (release, gate) = std::sync::mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = stop.clone();
            let thread = std::thread::spawn(move || {
                observe_sources(
                    &mut receiver,
                    "0123456789abcdef0123456789abcdef",
                    &stopped,
                    &crate::providers::memory::Memory::default(),
                    |_, _| {
                        entered.send(()).unwrap();
                        gate.recv_timeout(Duration::from_secs(1)).unwrap();
                        Ok(())
                    },
                )
            });
            peer.write_all(&packet("1")).unwrap();
            entry.recv_timeout(Duration::from_secs(1)).unwrap();
            if quit {
                stop.store(true, Ordering::SeqCst);
            } else {
                peer.shutdown(std::net::Shutdown::Both).unwrap();
            }
            release.send(()).unwrap();
            assert!(thread.join().unwrap().is_err());
            let mut ack = [0];
            assert!(peer.read_exact(&mut ack).is_err());
        }
    }
    #[test]
    fn source_framing_rejects_oversize_partial_and_total_read_deadline() {
        for length in [0u32, 262145] {
            let (mut peer, mut receiver) = UnixStream::pair().unwrap();
            peer.write_all(&length.to_be_bytes()).unwrap();
            assert!(bounded_frame(&mut receiver, 256 * 1024).is_err());
        }
        let (mut peer, mut receiver) = UnixStream::pair().unwrap();
        peer.write_all(&[0, 0]).unwrap();
        drop(peer);
        assert!(bounded_frame(&mut receiver, 256 * 1024).is_err());
        let (mut peer, mut receiver) = UnixStream::pair().unwrap();
        peer.write_all(&10u32.to_be_bytes()).unwrap();
        peer.write_all(b"half").unwrap();
        let at = Instant::now();
        assert!(bounded_frame(&mut receiver, 256 * 1024).is_err());
        assert!(at.elapsed() < Duration::from_secs(3));
    }
    #[test]
    fn late_dispatch_completion_is_not_acknowledged() {
        let (mut peer, mut receiver) = UnixStream::pair().unwrap();
        peer.write_all(&packet("1")).unwrap();
        assert!(
            observe_sources(
                &mut receiver,
                "0123456789abcdef0123456789abcdef",
                &AtomicBool::new(false),
                &crate::providers::memory::Memory::default(),
                |_, _| {
                    std::thread::sleep(Duration::from_millis(2050));
                    Ok(())
                }
            )
            .is_err()
        );
        peer.set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut ack = [0];
        assert!(peer.read_exact(&mut ack).is_err());
    }
}
