//! Adapter-owned payloads retain their reservation through queue/worker handoff.
use super::Failure;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
pub const ADAPTER_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone)]
pub struct Memory(Arc<Semaphore>);
impl Default for Memory {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(ADAPTER_BYTES)))
    }
}
impl Memory {
    pub fn same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn owns(&self, bytes: &Bytes) -> bool {
        Arc::ptr_eq(&self.0, bytes._charge.semaphore())
    }
    pub fn reserve(&self, bytes: usize) -> Result<OwnedSemaphorePermit, Failure> {
        self.0
            .clone()
            .try_acquire_many_owned(u32::try_from(bytes).map_err(|_| Failure::Capacity)?)
            .map_err(|_| Failure::Capacity)
    }
    pub fn used(&self) -> usize {
        ADAPTER_BYTES - self.0.available_permits()
    }
    pub(super) fn finish_receive(
        &self,
        data: Vec<u8>,
        mut charge: OwnedSemaphorePermit,
    ) -> Result<Bytes, Failure> {
        if !Arc::ptr_eq(&self.0, charge.semaphore()) {
            return Err(Failure::InvalidRequest);
        }
        let spare = charge
            .num_permits()
            .checked_sub(data.capacity())
            .ok_or(Failure::Capacity)?;
        drop(charge.split(spare).ok_or(Failure::Capacity)?);
        Ok(Bytes {
            data,
            _charge: charge,
        })
    }
    pub fn copy(&self, bytes: &[u8]) -> Result<Bytes, Failure> {
        let charge = self.reserve(bytes.len())?;
        Ok(Bytes {
            data: bytes.to_vec(),
            _charge: charge,
        })
    }
}
pub struct Bytes {
    pub(super) data: Vec<u8>,
    pub(super) _charge: OwnedSemaphorePermit,
}
impl Bytes {
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
}
