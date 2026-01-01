use std::fs;
use std::io::Result;

fn main() -> Result<()> {
    let out_path = "platforms/src/douyin/danmu/gen";

    // Ensure the output directory exists
    fs::create_dir_all(out_path)?;

    prost_build::Config::new()
        .out_dir(out_path) // Specify the output directory within the project
        .compile_protos(
            &["platforms/src/douyin/danmu/douyin.proto"], // Updated path after platforms directory move
            &["platforms/src/douyin/danmu/"], // Updated include path after platforms directory move
        )
        .expect("Failed to compile danmu protos");

    tauri_build::build(); // Call this if it's needed by your Tauri setup, otherwise can be removed if you handle tauri specific build steps elsewhere.

    Ok(())
}
