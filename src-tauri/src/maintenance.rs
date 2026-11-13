use std::sync::{
    atomic::{AtomicU64, Ordering},
    RwLock,
};
pub static ACCESS: RwLock<()> = RwLock::new(());
static GENERATION: AtomicU64 = AtomicU64::new(0);
pub fn generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}
