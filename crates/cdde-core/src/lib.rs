// Error types module
pub mod error;

// Diameter protocol module
pub mod diameter;

// Re-export commonly used types
pub use error::{CddeError, ErrorSeverity, Result};
pub use diameter::{DiameterHeader, DiameterAvp, DiameterPacket};
