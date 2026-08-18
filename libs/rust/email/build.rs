fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    tonic_prost_build::configure().compile_protos(
        &[
            "../../../contracts/protobuf/nvbes/platform/v1/common.proto",
            "../../../contracts/protobuf/nvbes/email/v1/email.proto",
        ],
        &["../../../contracts/protobuf"],
    )?;

    println!("cargo:rerun-if-changed=../../../contracts/protobuf/nvbes/platform/v1/common.proto");
    println!("cargo:rerun-if-changed=../../../contracts/protobuf/nvbes/email/v1/email.proto");
    Ok(())
}
