use std::path::PathBuf;

fn main() {
    // 编译proto文件
    let proto_file = PathBuf::from("src/rpc.proto");
    
    prost_build::compile_protos(&[proto_file], &["src/"])
        .unwrap_or_else(|e| panic!("Failed to compile proto files: {}", e));
    
    // 确保build.rs重新运行当proto文件更改时
    println!("cargo:rerun-if-changed=src/rpc.proto");
}
