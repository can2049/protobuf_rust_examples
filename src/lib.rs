//! 使用 protobuf **官方** Rust 实现（`google-protobuf` + `protoc --rust_out`）的示例集。
//!
//! 生成代码的模块名不可随意改动：codegen 会在生成文件内部用
//! `super::super::<proto 路径下划线化>::...` 做自引用（例如 `protos/person.proto`
//! 生成的内容里会引用 `protos_person_proto`）。因此要求：
//!
//! 1. 每个 proto 的生成文件必须包在**按路径下划线化命名**的模块里；
//! 2. 这些模块必须是**同一父模块**下的兄弟模块（这里统一放在 `pb` 下）。
//!
//! 破坏这两条会在编译期报「找不到模块」。
#[allow(
    nonstandard_style,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused
)]
pub mod pb {
    pub mod protos_person_proto {
        include!(concat!(
            env!("OUT_DIR"),
            "/protobuf_generated/protos/person.u.pb.rs"
        ));
    }

    pub mod protos_catalog_proto {
        include!(concat!(
            env!("OUT_DIR"),
            "/protobuf_generated/protos/catalog.u.pb.rs"
        ));
    }

    pub mod protos_scalars_proto {
        include!(concat!(
            env!("OUT_DIR"),
            "/protobuf_generated/protos/scalars.u.pb.rs"
        ));
    }

    pub mod protos_wkt_proto {
        include!(concat!(
            env!("OUT_DIR"),
            "/protobuf_generated/protos/wkt.u.pb.rs"
        ));
    }

    pub mod protos_log_proto {
        include!(concat!(
            env!("OUT_DIR"),
            "/protobuf_generated/protos/log.u.pb.rs"
        ));
    }

    // 上面的模块名由 codegen 约束；下面的别名只是让示例代码好读。
    // 例如 `pb::person_pb::Person`。注意生成代码内部还有同名的 `person` 子模块
    // （放 map entry / oneof / 嵌套 message），所以这里不直接用 `person` 做别名。
    pub use protos_catalog_proto as catalog_pb;
    pub use protos_log_proto as log_pb;
    pub use protos_person_proto as person_pb;
    pub use protos_scalars_proto as scalars_pb;
    pub use protos_wkt_proto as wkt_pb;
}
