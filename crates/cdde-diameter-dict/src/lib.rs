// Diameter dictionary module
pub mod data_type;
pub mod manager;
pub mod standard;

// Re-export commonly used types
pub use data_type::{AvpDataType, AvpValue, ParseError};
pub use manager::{AvpInfo, DictionaryManager};
pub use standard::StandardAvpCode;
