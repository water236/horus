/// Parallel executor for running independent nodes concurrently
pub mod parallel;

/// Async I/O executor for non-blocking operations
pub mod async_io;

/// Background executor for low-priority node execution
pub mod background;

pub use async_io::{AsyncIOExecutor, AsyncResult};
pub use background::BackgroundExecutor;
pub use parallel::ParallelExecutor;
