// build.rs
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Re-run build.rs if these change:
    println!("cargo:rerun-if-changed=bazel_output_base.txt");
    println!("cargo:rerun-if-env-changed=ENVOY_API_DIR");

    // 1) read where Bazel has checked out its externals:
    let bazel_base = fs::read_to_string("bazel_output_base.txt")?
        .trim()
        .to_string();

    // 2) where your envoy repo lives (contains envoy/... protos):
    let api_root = PathBuf::from(env::var("ENVOY_API_DIR")?);

    // 3) your *entry* proto(s):
    let protos = vec![
        api_root.join("envoy/service/ext_proc/v3/external_processor.proto"),
    ];

    // 4) every directory you want protoc to search for imports:
    let includes = vec![
        api_root.clone(),                                                 // your envoy/api
        PathBuf::from(&bazel_base).join("external/com_google_protobuf/src"),
        PathBuf::from(&bazel_base).join("external/com_google_googleapis"),
        PathBuf::from(&bazel_base).join("external/com_envoyproxy_protoc_gen_validate"),
        PathBuf::from(&bazel_base).join("external/com_github_cncf_xds"),
    ];

    // Determine the OUT_DIR for the descriptor set path
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_well_known_types(true)
        // Add this line to generate a single descriptor file:
        .file_descriptor_set_path(out_dir.join("ext_proc_descriptor.bin")) // You can name the .bin file anything
        .compile_protos(
            &protos.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
            &includes.iter().map(|i| i.as_path()).collect::<Vec<_>>(),
        )?;

    Ok(())
}
