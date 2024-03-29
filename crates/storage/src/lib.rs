// pub mod convert;
// pub mod entities;
pub mod entity;
pub mod migration;
pub mod query;
pub mod storage;

use std::sync::OnceLock;

use tokio::runtime::Runtime;

#[allow(clippy::expect_used)]
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}
