// Error types module
pub mod error;

// Diameter protocol module
pub mod diameter;

// Transport abstraction module
pub mod transport;

// Re-export commonly used types
pub use diameter::{DiameterAvp, DiameterHeader, DiameterPacket};
pub use error::{CddeError, ErrorSeverity, Result};
pub use transport::Transport;
