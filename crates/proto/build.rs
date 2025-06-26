use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from("src/generated");

    // Создаём папку, если её нет
    fs::create_dir_all(&out_dir)?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&out_dir)
        .compile(&["conversation.proto"], &["."])?;

    println!("cargo:rerun-if-changed=conversation.proto");
    println!("cargo:rerun-if-changed=.");

    Ok(())
}
