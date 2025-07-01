fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "src/proto/node.proto",
                "src/proto/file.proto", 
                "src/proto/log.proto",
            ],
            &["src/proto"],
        )?;
    Ok(())
}