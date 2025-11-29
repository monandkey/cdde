fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile(
            &["proto/cdde.proto", "proto/internal.proto"],
            &["proto"],
        )?;
    Ok(())
}
