use std::path::{Path, PathBuf};

/// 必须与 Cargo.toml 里的 protobuf crate 版本对齐。
/// 官方规则：crate 4.36.2-release <-> protoc 36.2（去掉首段与 -release 后缀）。
/// 升级版本请用 `bash scripts/update-protobuf.sh`，它会同步改写这里。
const PROTOC_VERSION: &str = "36.2";

/// proto 路径相对于 --proto_path（即 crate 根目录），import 语句也按这个全路径写
const PROTO_FILES: [&str; 5] = [
    "protos/person.proto",
    "protos/catalog.proto",
    "protos/scalars.proto",
    "protos/wkt.proto",
    "protos/log.proto",
];

fn find_protoc(manifest_dir: &Path) -> PathBuf {
    if let Some(from_env) = std::env::var_os("PROTOC") {
        return PathBuf::from(from_env);
    }
    let vendored = manifest_dir
        .join("tools")
        .join(format!("protoc-{PROTOC_VERSION}"))
        .join("bin/protoc");
    if vendored.exists() {
        return vendored;
    }
    PathBuf::from("protoc")
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    for proto in PROTO_FILES {
        println!("cargo:rerun-if-changed={proto}");
    }

    protobuf_codegen::CodeGen::new()
        .protoc_path(find_protoc(&manifest_dir))
        .include(&manifest_dir)
        // 复用 WKT crate 自带的 proto 定义与其依赖清单，
        // 生成的代码会以 ::protobuf_well_known_types:: 路径引用 Timestamp 等类型
        .dependency(protobuf_well_known_types::get_dependency(
            "protobuf_well_known_types",
        ))
        .inputs(PROTO_FILES)
        .generate_and_compile()
        .unwrap_or_else(|e| {
            panic!(
                "protobuf 代码生成失败: {e}\n\
                 protoc 版本必须与 Cargo.toml 里 pin 的 protobuf crate 精确匹配\n\
                 （crate 版本去掉首段与 -release 后缀即为所需 protoc 版本）。\n\
                 本次期望 protoc {PROTOC_VERSION}，请先执行: bash scripts/install-protoc.sh\n\
                 或设置环境变量 PROTOC 指向匹配版本的 protoc。"
            )
        });
}
