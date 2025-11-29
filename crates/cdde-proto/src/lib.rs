pub mod cdde {
    tonic::include_proto!("cdde");
}

pub mod internal {
    tonic::include_proto!("cdde.internal");
}

// Re-export ActionType for convenience if needed,
// though it's now part of the generated module.
// We can add helper methods here if necessary.
