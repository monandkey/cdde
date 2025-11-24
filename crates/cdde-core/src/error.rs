use thiserror::Error;

/// Main error type for CDDE system
#[derive(Error, Debug)]
pub enum CddeError {
    // ========================================
    // Protocol Errors
    // ========================================
    #[error("Invalid Diameter packet: {0}")]
    InvalidPacket(String),

    #[error("Missing required AVP: {0}")]
    MissingAvp(u32),

    #[error("Invalid AVP value for code {code}: {reason}")]
    InvalidAvpValue { code: u32, reason: String },

    // ========================================
    // Routing Errors
    // ========================================
    #[error("No route found for realm: {0}")]
    NoRoute(String),

    #[error("All peers are down for pool: {0}")]
    AllPeersDown(String),

    #[error("Routing loop detected")]
    RoutingLoop,

    // ========================================
    // Timeout Errors
    // ========================================
    #[error("Session timeout after {0}ms")]
    SessionTimeout(u64),

    #[error("gRPC call timeout")]
    GrpcTimeout,

    // ========================================
    // System Errors
    // ========================================
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    // ========================================
    // Network Errors
    // ========================================
    #[error("SCTP connection failed: {0}")]
    SctpError(#[from] std::io::Error),

    #[error("Network error: {0}")]
    NetworkError(String),
}

impl CddeError {
    /// Convert error to Diameter Result-Code
    pub fn to_result_code(&self) -> u32 {
        match self {
            Self::InvalidPacket(_) => 3008, // DIAMETER_INVALID_AVP_VALUE
            Self::MissingAvp(_) => 5005,    // DIAMETER_MISSING_AVP
            Self::InvalidAvpValue { .. } => 3008,
            Self::NoRoute(_) => 3003,      // DIAMETER_REALM_NOT_SERVED
            Self::AllPeersDown(_) => 3002, // DIAMETER_UNABLE_TO_DELIVER
            Self::RoutingLoop => 3005,     // DIAMETER_LOOP_DETECTED
            Self::SessionTimeout(_) => 3002,
            Self::GrpcTimeout => 3002,
            _ => 3010, // DIAMETER_UNABLE_TO_COMPLY
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::InvalidPacket(_) | Self::MissingAvp(_) => ErrorSeverity::Warning,
            Self::NoRoute(_) | Self::AllPeersDown(_) => ErrorSeverity::Error,
            Self::RoutingLoop => ErrorSeverity::Critical,
            Self::InternalError(_) => ErrorSeverity::Critical,
            _ => ErrorSeverity::Warning,
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::GrpcTimeout | Self::SctpError(_) | Self::NetworkError(_)
        )
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Result type alias for CDDE operations
pub type Result<T> = std::result::Result<T, CddeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_result_code() {
        assert_eq!(
            CddeError::InvalidPacket("test".to_string()).to_result_code(),
            3008
        );
        assert_eq!(CddeError::MissingAvp(264).to_result_code(), 5005);
        assert_eq!(
            CddeError::NoRoute("test.realm".to_string()).to_result_code(),
            3003
        );
        assert_eq!(CddeError::RoutingLoop.to_result_code(), 3005);
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(
            CddeError::InvalidPacket("test".to_string()).severity(),
            ErrorSeverity::Warning
        );
        assert_eq!(CddeError::RoutingLoop.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_retryable() {
        assert!(CddeError::GrpcTimeout.is_retryable());
        assert!(!CddeError::RoutingLoop.is_retryable());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(ErrorSeverity::Info.to_string(), "info");
        assert_eq!(ErrorSeverity::Critical.to_string(), "critical");
    }
}
