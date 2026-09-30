//! 场景 2：全部标量类型 + proto3 零值语义 + `optional` 字段的显式 presence。
//!
//! 运行：`cargo run --example 02_scalars_presence`
use protobuf::prelude::*;
use protobuf_rust_examples::pb::scalars_pb::{AllScalars, PresenceDemo};

fn main() {
    let mut s = AllScalars::new();
    s.set_double_val(3.5);
    s.set_float_val(1.25);
    s.set_int32_val(-7);
    s.set_int64_val(-1_234_567_890_123);
    s.set_uint32_val(7);
    s.set_uint64_val(u64::MAX);
    s.set_sint32_val(-7);
    s.set_sint64_val(-1_234_567_890_123);
    s.set_fixed32_val(7);
    s.set_fixed64_val(u64::MAX);
    s.set_sfixed32_val(-7);
    s.set_sfixed64_val(-1_234_567_890_123);
    s.set_bool_val(true);
    s.set_string_val("hello, 世界");
    s.set_bytes_val(vec![0u8, 1, 2, 255]);

    let bytes = s.serialize().unwrap();
    println!("AllScalars wire = {} bytes", bytes.len());

    let back = AllScalars::parse(&bytes).unwrap();
    println!("double  = {}", back.double_val());
    println!("float   = {}", back.float_val());
    println!("int32   = {}", back.int32_val());
    println!("int64   = {}", back.int64_val());
    println!("uint32  = {}", back.uint32_val());
    println!("uint64  = {}", back.uint64_val());
    println!("sint32  = {}", back.sint32_val());
    println!("sint64  = {}", back.sint64_val());
    println!("fixed32 = {}", back.fixed32_val());
    println!("fixed64 = {}", back.fixed64_val());
    println!("sfixed32= {}", back.sfixed32_val());
    println!("sfixed64= {}", back.sfixed64_val());
    println!("bool    = {}", back.bool_val());
    println!("string  = {}", back.string_val());
    println!("bytes   = {:?}", back.bytes_val().as_view());

    println!("\n--- proto3 零值不上 wire ---");
    let mut zero = AllScalars::new();
    zero.set_int32_val(0);
    zero.set_string_val("");
    zero.set_bool_val(false);
    let zero_bytes = zero.serialize().unwrap();
    println!("只设了 0 / \"\" / false -> {} bytes {zero_bytes:?}", zero_bytes.len());
    println!("解析回来 int32_val = {}", AllScalars::parse(&zero_bytes).unwrap().int32_val());

    println!("\n--- optional：区分「没设置」和「设成零值」---");
    let mut d = PresenceDemo::new();
    d.set_plain_int32(0);
    println!(
        "普通 int32 设 0：has_opt_int32={} wire={} bytes",
        d.has_opt_int32(),
        d.serialize().unwrap().len()
    );
    d.set_opt_int32(0);
    println!(
        "optional 设 0：has_opt_int32={} wire={} bytes（0 也会被写进去）",
        d.has_opt_int32(),
        d.serialize().unwrap().len()
    );
    println!("optional 读值：opt_int32={}", d.opt_int32());
    d.clear_opt_int32();
    println!("clear_opt_int32() 之后：has={}", d.has_opt_int32());

    let unset_bool = PresenceDemo::new().opt_bool_opt();
    let mut with_false = PresenceDemo::new();
    with_false.set_opt_bool(false);
    println!(
        "optional bool 未设 -> {unset_bool:?}；设成 false -> {:?}",
        with_false.opt_bool_opt()
    );
}
