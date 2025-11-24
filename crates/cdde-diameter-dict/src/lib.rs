// Diameter dictionary module
pub mod standard;
pub mod data_type;
pub mod manager;

// Re-export commonly used types
pub use standard::StandardAvpCode;
pub use data_type::{AvpDataType, AvpValue, ParseError};
pub use manager::{DictionaryManager, AvpInfo};
