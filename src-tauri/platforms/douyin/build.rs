use std::fs;
use std::io::Result;

fn main() -> Result<()> {
    let out_path = "src/message/gen";

    // Ensure the output directory exists
    fs::create_dir_all(out_path)?;

    prost_build::Config::new()
        .out_dir(out_path) // Specify the output directory within the project
        .type_attribute(".", "#[derive(serde::Serialize)]") // Add serde Serialize support
        .compile_protos(
            &["src/message/douyin.proto"], // Path relative to the douyin platform directory
            &["src/message/"], // Include path relative to the douyin platform directory
        )
        .expect("Failed to compile message protos");

    Ok(())
}