//! Server Signal
//!
//! Server-side reactive signal implementation

pub mod signal;
pub mod traits;

// Re-export main types
pub use signal::ServerSignal;
pub use traits::ServerSignalTrait;
