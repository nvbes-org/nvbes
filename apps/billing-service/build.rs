fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    tonic_prost_build::configure()
        .type_attribute(".", "#[allow(dead_code)]")
        .compile_protos(
            &[
                "../../contracts/protobuf/nvbes/platform/v1/common.proto",
                "../../contracts/protobuf/nvbes/identity/internal/v1/identity_internal.proto",
                "../../contracts/protobuf/nvbes/billing/v1/billing.proto",
            ],
            &["../../contracts/protobuf"],
        )?;

    println!("cargo:rerun-if-changed=../../contracts/protobuf/nvbes/platform/v1/common.proto");
    println!(
        "cargo:rerun-if-changed=../../contracts/protobuf/nvbes/identity/internal/v1/identity_internal.proto"
    );
    println!("cargo:rerun-if-changed=../../contracts/protobuf/nvbes/billing/v1/billing.proto");
    Ok(())
}
