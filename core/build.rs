fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/proto");
    tonic_build::compile_protos("src/proto/node.proto")?;
    Ok(())
} 