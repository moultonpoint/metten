//! Fixed protected records. No arbitrary secret read or native command.
use crate::providers::{Environment, Failure, signing::Key};
use serde::{Deserialize, Serialize};
const RECORD_BYTES: usize = 20 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u8,
    key_id: String,
    private_key: String,
}
pub fn valid_id(id: &str) -> bool {
    (1..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii() && !b.is_ascii_control())
}
impl Record {
    fn validate(&self) -> Result<Key, Failure> {
        if self.version != 1 || !valid_id(&self.key_id) {
            return Err(Failure::InvalidCredential);
        }
        Key::parse(&self.private_key)
    }
    fn decode(bytes: &[u8]) -> Result<Self, Failure> {
        if bytes.len() > RECORD_BYTES {
            return Err(Failure::InvalidCredential);
        }
        let record: Self = serde_json::from_slice(bytes).map_err(|_| Failure::InvalidCredential)?;
        record.validate()?;
        Ok(record)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Missing,
    Saved {
        key_id: String,
        key_type: &'static str,
    },
}
#[derive(Clone, Copy)]
pub enum Mutation {
    Save,
    Replace,
    Remove,
}
// A store seam is required for deterministic failure/ambiguous-outcome tests.
pub trait Store {
    fn read(&mut self, environment: Environment) -> Result<Option<Vec<u8>>, Failure>;
    fn mutate(
        &mut self,
        environment: Environment,
        operation: Mutation,
        record: Option<&[u8]>,
    ) -> Result<(), Failure>;
}
pub struct Credentials<S> {
    store: S,
}
impl<S: Store> Credentials<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
    pub fn status(&mut self, environment: Environment) -> Result<Status, Failure> {
        match self.store.read(environment)? {
            None => Ok(Status::Missing),
            Some(bytes) => {
                let record = Record::decode(&bytes)?;
                Ok(Status::Saved {
                    key_type: record.validate()?.kind(),
                    key_id: record.key_id,
                })
            }
        }
    }
    pub fn change(
        &mut self,
        environment: Environment,
        operation: Mutation,
        material: Option<(&str, &str)>,
        owner: &mut crate::providers::Coordinator,
    ) -> Result<(), Failure> {
        let bytes = match operation {
            Mutation::Save | Mutation::Replace => {
                let (id, pem) = material.ok_or(Failure::InvalidCredential)?;
                if !valid_id(id) || pem.len() > 16384 {
                    return Err(Failure::InvalidCredential);
                }
                let record = Record {
                    version: 1,
                    key_id: id.into(),
                    private_key: pem.into(),
                };
                record.validate()?;
                Some(serde_json::to_vec(&record).map_err(|_| Failure::InvalidCredential)?)
            }
            Mutation::Remove if material.is_none() => None,
            _ => return Err(Failure::InvalidRequest),
        };
        if bytes
            .as_ref()
            .is_some_and(|bytes| bytes.len() > RECORD_BYTES)
        {
            return Err(Failure::InvalidCredential);
        }
        // Called on the native blocking worker with exclusive moved ownership, not under a shared lock.
        if !matches!(operation, Mutation::Save) {
            tauri::async_runtime::block_on(owner.stop_provider(
                crate::providers::Provider::Kalshi,
                tokio::time::Instant::now() + crate::providers::SHUTDOWN,
            ))?;
        }
        let previous = self.store.read(environment)?;
        match operation {
            Mutation::Save if previous.is_some() => return Err(Failure::AlreadySaved),
            Mutation::Replace | Mutation::Remove if previous.is_none() => {
                return Err(Failure::Missing);
            }
            _ => (),
        }
        let outcome = self.store.mutate(environment, operation, bytes.as_deref());
        let actual = self
            .store
            .read(environment)
            .map_err(|_| Failure::Ambiguous)?;
        if actual.as_deref() == bytes.as_deref() && outcome.is_ok() {
            return Ok(());
        }
        if let Err(error) = outcome {
            return Err(if actual == previous {
                error
            } else {
                Failure::Ambiguous
            });
        }
        Err(Failure::Ambiguous)
    }
    // Trusted native transport alone consumes this material; no IPC representation.
    pub fn with_signer<T>(
        &mut self,
        environment: Environment,
        use_key: impl FnOnce(&str, &Key) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        let bytes = self.store.read(environment)?.ok_or(Failure::Missing)?;
        let record = Record::decode(&bytes)?;
        use_key(&record.key_id, &record.validate()?)
    }
}

pub mod native {
    use super::*;
    use core_foundation::array::CFArray;
    use core_foundation::{
        base::{CFType, TCFType},
        boolean::CFBoolean,
        data::CFData,
        dictionary::CFDictionary,
        string::CFString,
    };
    use security_framework_sys::{
        access_control::kSecAttrAccessibleWhenUnlockedThisDeviceOnly, item::*, keychain_item::*,
    };
    unsafe extern "C" {
        static kSecAttrAccessible: core_foundation::string::CFStringRef;
        static kSecMatchLimitOne: core_foundation::string::CFStringRef;
    }
    pub struct Keychain {
        isolated: Option<crate::credential::Bootstrap>,
    }
    impl Keychain {
        pub fn local() -> Self {
            Self { isolated: None }
        }
    }
    fn error(status: i32) -> Result<(), Failure> {
        match status {
            0 => Ok(()),
            -25300 => Err(Failure::Missing),
            -25299 => Err(Failure::AlreadySaved),
            _ => Err(Failure::Unavailable),
        }
    }
    impl Keychain {
        pub(crate) fn isolated(target: crate::credential::Bootstrap) -> Self {
            Self {
                isolated: Some(target),
            }
        }
        unsafe fn pairs(
            &self,
            environment: Environment,
            add: bool,
        ) -> Result<Vec<(CFType, CFType)>, Failure> {
            unsafe {
                let mut pairs = vec![
                    (
                        CFString::wrap_under_get_rule(kSecClass).as_CFType(),
                        CFString::wrap_under_get_rule(kSecClassGenericPassword).as_CFType(),
                    ),
                    (
                        CFString::wrap_under_get_rule(kSecAttrService).as_CFType(),
                        CFString::new("com.metten.provider-credential").as_CFType(),
                    ),
                    (
                        CFString::wrap_under_get_rule(kSecAttrAccount).as_CFType(),
                        CFString::new(environment.account()).as_CFType(),
                    ),
                    (
                        CFString::wrap_under_get_rule(kSecAttrSynchronizable).as_CFType(),
                        CFBoolean::false_value().as_CFType(),
                    ),
                ];
                if let Some(target) = &self.isolated {
                    let keychain = target.open().map_err(|_| Failure::Unavailable)?;
                    crate::credential::unlocked(&keychain).map_err(|_| Failure::Unavailable)?;
                    pairs.push((
                        CFString::wrap_under_get_rule(kSecUseDataProtectionKeychain).as_CFType(),
                        CFBoolean::false_value().as_CFType(),
                    ));
                    pairs.push(if add {
                        (
                            CFString::wrap_under_get_rule(kSecUseKeychain).as_CFType(),
                            keychain.clone(),
                        )
                    } else {
                        (
                            CFString::wrap_under_get_rule(kSecMatchSearchList).as_CFType(),
                            CFArray::from_CFTypes(std::slice::from_ref(&keychain)).as_CFType(),
                        )
                    });
                    return Ok(pairs);
                }
                let _ = add;
                pairs.push((
                    CFString::wrap_under_get_rule(kSecUseDataProtectionKeychain).as_CFType(),
                    CFBoolean::true_value().as_CFType(),
                ));
                Ok(pairs)
            }
        }
    }
    impl Store for Keychain {
        fn read(&mut self, environment: Environment) -> Result<Option<Vec<u8>>, Failure> {
            unsafe {
                let mut pairs = self.pairs(environment, false)?;
                pairs.push((
                    CFString::wrap_under_get_rule(kSecReturnData).as_CFType(),
                    CFBoolean::true_value().as_CFType(),
                ));
                pairs.push((
                    CFString::wrap_under_get_rule(kSecMatchLimit).as_CFType(),
                    CFString::wrap_under_get_rule(kSecMatchLimitOne).as_CFType(),
                ));
                let mut raw = std::ptr::null();
                let status = SecItemCopyMatching(
                    CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef(),
                    &mut raw,
                );
                if status == -25300 {
                    return Ok(None);
                }
                error(status)?;
                if raw.is_null() {
                    return Err(Failure::Unavailable);
                }
                let data = CFType::wrap_under_create_rule(raw)
                    .downcast::<CFData>()
                    .ok_or(Failure::InvalidCredential)?;
                if data.len() as usize > RECORD_BYTES {
                    return Err(Failure::InvalidCredential);
                }
                Ok(Some(data.bytes().to_vec()))
            }
        }
        fn mutate(
            &mut self,
            environment: Environment,
            operation: Mutation,
            record: Option<&[u8]>,
        ) -> Result<(), Failure> {
            unsafe {
                let mut pairs = self.pairs(environment, matches!(operation, Mutation::Save))?;
                let result = match operation {
                    Mutation::Save => {
                        pairs.push((
                            CFString::wrap_under_get_rule(kSecAttrAccessible).as_CFType(),
                            CFString::wrap_under_get_rule(
                                kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
                            )
                            .as_CFType(),
                        ));
                        pairs.push((
                            CFString::wrap_under_get_rule(kSecValueData).as_CFType(),
                            CFData::from_buffer(record.ok_or(Failure::InvalidRequest)?).as_CFType(),
                        ));
                        SecItemAdd(
                            CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef(),
                            std::ptr::null_mut(),
                        )
                    }
                    Mutation::Replace => {
                        let update = CFDictionary::from_CFType_pairs(&[(
                            CFString::wrap_under_get_rule(kSecValueData),
                            CFData::from_buffer(record.ok_or(Failure::InvalidRequest)?),
                        )]);
                        SecItemUpdate(
                            CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef(),
                            update.as_concrete_TypeRef(),
                        )
                    }
                    Mutation::Remove => {
                        SecItemDelete(CFDictionary::from_CFType_pairs(&pairs).as_concrete_TypeRef())
                    }
                };
                error(result)?;
                if let Some(target) = &mut self.isolated {
                    target
                        .record_native_write()
                        .map_err(|_| Failure::Ambiguous)?;
                }
                Ok(())
            }
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        #[ignore = "Creates only an owned disposable Keychain; run explicitly on macOS"]
        fn isolated_native_fixed_items() {
            use std::os::unix::fs::PermissionsExt;
            let directory = std::env::temp_dir()
                .join(format!("metten-credential-provider-{}", std::process::id()));
            std::fs::create_dir(&directory).unwrap();
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
            let path = directory.join("dummy.keychain-db");
            let password = "metten-disposable-controlled-test-only";
            use security_framework_sys::keychain::SecKeychainCreate;
            unsafe extern "C" {
                fn SecKeychainDelete(keychain: security_framework_sys::base::SecKeychainRef)
                -> i32;
            }
            let mut raw = std::ptr::null_mut();
            let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            unsafe {
                assert_eq!(
                    SecKeychainCreate(
                        path.as_ptr(),
                        password.len() as u32,
                        password.as_ptr().cast(),
                        0,
                        std::ptr::null_mut(),
                        &mut raw
                    ),
                    0
                );
            }
            struct Cleanup(
                std::path::PathBuf,
                security_framework_sys::base::SecKeychainRef,
            );
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    unsafe {
                        assert_eq!(SecKeychainDelete(self.1), 0);
                    }
                    let _ = std::fs::remove_dir_all(&self.0);
                }
            }
            let _cleanup = Cleanup(directory, raw);
            let _keychain = unsafe { CFType::wrap_under_create_rule(raw.cast()) };
            let mut store = Keychain {
                isolated: Some(crate::credential::Bootstrap::fixture(
                    _cleanup.0.join("dummy.keychain-db"),
                )),
            };
            for environment in [Environment::Demo, Environment::Production] {
                assert!(store.read(environment).unwrap().is_none());
                store
                    .mutate(environment, Mutation::Save, Some(b"controlled-A"))
                    .unwrap();
                assert_eq!(
                    store.mutate(environment, Mutation::Save, Some(b"bad")),
                    Err(Failure::AlreadySaved)
                );
                assert_eq!(store.read(environment).unwrap().unwrap(), b"controlled-A");
                store
                    .mutate(environment, Mutation::Replace, Some(b"controlled-B"))
                    .unwrap();
                assert_eq!(store.read(environment).unwrap().unwrap(), b"controlled-B");
                store.mutate(environment, Mutation::Remove, None).unwrap();
                assert!(store.read(environment).unwrap().is_none());
                assert_eq!(
                    store.mutate(environment, Mutation::Remove, None),
                    Err(Failure::Missing)
                );
            }
        }
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;
    #[test]
    fn strict_records_and_failed_mutations_preserve_previous_material() {
        use ed25519_dalek::pkcs8::EncodePrivateKey;
        let pem = ed25519_dalek::SigningKey::from_bytes(&[9; 32])
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap()
            .to_string();
        for bytes in [
            b"{}".as_slice(),
            b"{\"version\":2,\"key_id\":\"id\",\"private_key\":\"bad\"}",
            b"{\"version\":1,\"version\":1,\"key_id\":\"id\",\"private_key\":\"bad\"}",
        ] {
            assert!(Record::decode(bytes).is_err());
        }
        let previous = serde_json::to_vec(&Record {
            version: 1,
            key_id: "old".into(),
            private_key: pem.clone(),
        })
        .unwrap();
        struct Failed(Vec<u8>);
        impl Store for Failed {
            fn read(&mut self, _: Environment) -> Result<Option<Vec<u8>>, Failure> {
                Ok(Some(self.0.clone()))
            }
            fn mutate(
                &mut self,
                _: Environment,
                _: Mutation,
                _: Option<&[u8]>,
            ) -> Result<(), Failure> {
                Err(Failure::Unavailable)
            }
        }
        let mut credentials = Credentials::new(Failed(previous));
        let mut owner = crate::providers::Coordinator::new();
        owner.revoke();
        assert_eq!(
            credentials.change(
                Environment::Demo,
                Mutation::Replace,
                Some(("new", &pem)),
                &mut owner
            ),
            Err(Failure::Unavailable)
        );
        assert_eq!(
            credentials.change(Environment::Demo, Mutation::Remove, None, &mut owner),
            Err(Failure::Unavailable)
        );
        assert_eq!(
            credentials.status(Environment::Demo),
            Ok(Status::Saved {
                key_id: "old".into(),
                key_type: "ed25519"
            })
        );
    }
}

#[cfg(test)]
mod stop_tests {
    use super::*;
    use crate::providers::{Coordinator, Provider};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    #[test]
    fn replacement_validates_then_joins_shared_kalshi_work_without_stopping_spot() {
        use ed25519_dalek::pkcs8::EncodePrivateKey;
        let pem = ed25519_dalek::SigningKey::from_bytes(&[10; 32])
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap()
            .to_string();
        let exited = Arc::new(AtomicBool::new(false));
        struct Memory {
            bytes: Vec<u8>,
            exited: Arc<AtomicBool>,
        }
        impl Store for Memory {
            fn read(&mut self, _: Environment) -> Result<Option<Vec<u8>>, Failure> {
                Ok(Some(self.bytes.clone()))
            }
            fn mutate(
                &mut self,
                _: Environment,
                _: Mutation,
                bytes: Option<&[u8]>,
            ) -> Result<(), Failure> {
                assert!(self.exited.load(Ordering::SeqCst));
                self.bytes = bytes.unwrap().to_vec();
                Ok(())
            }
        }
        let bytes = serde_json::to_vec(&Record {
            version: 1,
            key_id: "old".into(),
            private_key: pem.clone(),
        })
        .unwrap();
        let mut credentials = Credentials::new(Memory {
            bytes,
            exited: exited.clone(),
        });
        let mut owner = Coordinator::new();
        let old = owner.fence(Provider::Kalshi);
        let spot = owner.fence(Provider::Coinbase);
        let mut cancel = owner.cancellation(Provider::Kalshi);
        tauri::async_runtime::block_on(async {
            owner
                .spawn(Provider::Kalshi, async move {
                    let _ = cancel.changed().await;
                    exited.store(true, Ordering::SeqCst);
                })
                .unwrap();
        });
        assert_eq!(
            credentials.change(
                Environment::Demo,
                Mutation::Replace,
                Some(("new", "bad")),
                &mut owner
            ),
            Err(Failure::InvalidCredential)
        );
        assert!(old.current(old.token()));
        credentials
            .change(
                Environment::Demo,
                Mutation::Replace,
                Some(("new", &pem)),
                &mut owner,
            )
            .unwrap();
        assert!(!old.current(old.token()));
        assert!(spot.current(spot.token()));
        owner.recover_provider(Provider::Kalshi, false).unwrap();
        assert!(!old.current(old.token()));
        assert!(owner.fence(Provider::Kalshi).current(1));
    }
}

pub(crate) fn verify_packaged(target: crate::credential::Bootstrap) -> Result<(), Failure> {
    use ed25519_dalek::pkcs8::EncodePrivateKey;
    let a = ed25519_dalek::SigningKey::from_bytes(&[41; 32])
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|_| Failure::InvalidCredential)?;
    let b = ed25519_dalek::SigningKey::from_bytes(&[42; 32])
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|_| Failure::InvalidCredential)?;
    let mut credentials = Credentials::new(native::Keychain::isolated(target));
    let mut owner = crate::providers::Coordinator::new();
    for environment in [Environment::Demo, Environment::Production] {
        if credentials.status(environment)? != Status::Missing {
            return Err(Failure::Protocol);
        }
        credentials.change(
            environment,
            Mutation::Save,
            Some(("controlled-A", &a)),
            &mut owner,
        )?;
        if credentials.change(
            environment,
            Mutation::Save,
            Some(("controlled-B", &b)),
            &mut owner,
        ) != Err(Failure::AlreadySaved)
        {
            return Err(Failure::Protocol);
        }
        credentials.change(
            environment,
            Mutation::Replace,
            Some(("controlled-B", &b)),
            &mut owner,
        )?;
        if credentials.status(environment)?
            != (Status::Saved {
                key_id: "controlled-B".into(),
                key_type: "ed25519",
            })
        {
            return Err(Failure::Protocol);
        }
        credentials.with_signer(environment, |id, key| {
            if id != "controlled-B" {
                return Err(Failure::Protocol);
            }
            let signature = key.sign(&crate::providers::Route::Socket, 123)?;
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(signature)
                .map_err(|_| Failure::Signing)?;
            ed25519_dalek::SigningKey::from_bytes(&[42; 32])
                .verifying_key()
                .verify_strict(
                    b"123GET/trade-api/ws/v2",
                    &ed25519_dalek::Signature::from_slice(&bytes).map_err(|_| Failure::Signing)?,
                )
                .map_err(|_| Failure::Signing)
        })?;
        credentials.change(environment, Mutation::Remove, None, &mut owner)?;
        if credentials.status(environment)? != Status::Missing {
            return Err(Failure::Protocol);
        }
    }
    Ok(())
}
