fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/solar-system-info/solar-system-info.proto")?;
    Ok(())
}
