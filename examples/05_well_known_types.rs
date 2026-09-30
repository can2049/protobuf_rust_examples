//! 场景 5：常用 well-known types（WKT）。
//!
//! 绑定来自 `protobuf-well-known-types` crate，类型名直接挂在 crate 根上。
//! 注意：官方运行时目前**没有** Any 的 pack/unpack 辅助，type_url 与 value 需要自己填。
//!
//! 运行：`cargo run --example 05_well_known_types`
use protobuf::prelude::*;
use protobuf_well_known_types::{
    Any, Duration, Empty, FieldMask, Int32Value, ListValue, StringValue, Struct, Timestamp, Value,
};
use protobuf_rust_examples::pb::wkt_pb::AnyThing;

fn main() {
    // --- Timestamp / Duration ---
    let mut ts = Timestamp::new();
    ts.set_seconds(1_700_000_000);
    ts.set_nanos(123_000_000);
    let ts_bytes = ts.serialize().unwrap();
    let ts2 = Timestamp::parse(&ts_bytes).unwrap();
    println!(
        "Timestamp: seconds={} nanos={} ({} bytes)",
        ts2.seconds(),
        ts2.nanos(),
        ts_bytes.len()
    );

    let mut dur = Duration::new();
    dur.set_seconds(90);
    dur.set_nanos(500_000_000);
    println!("Duration: {}s + {}ns", dur.seconds(), dur.nanos());

    // --- Struct：JSON 式的任意结构，由 Struct / Value / ListValue 组成 ---
    let mut obj = Struct::new();
    let mut name = Value::new();
    name.set_string_value("Alice");
    obj.fields_mut().insert("name", name);

    let mut age = Value::new();
    age.set_number_value(30.0);
    obj.fields_mut().insert("age", age);

    let mut hobbies = ListValue::new();
    let mut h1 = Value::new();
    h1.set_string_value("rust");
    let mut h2 = Value::new();
    h2.set_bool_value(true);
    hobbies.values_mut().push(h1);
    hobbies.values_mut().push(h2);
    let mut hobbies_value = Value::new();
    hobbies_value.set_list_value(hobbies);
    obj.fields_mut().insert("hobbies", hobbies_value);

    // --- Wrappers / FieldMask / Empty ---
    let mut sv = StringValue::new();
    sv.set_value("hello");
    let mut iv = Int32Value::new();
    iv.set_value(-1);
    let mut mask = FieldMask::new();
    mask.paths_mut().push("name");
    mask.paths_mut().push("scores.math");

    // --- Any：手工拼 type_url + value ---
    let mut any = Any::new();
    any.set_type_url("type.googleapis.com/google.protobuf.Timestamp");
    any.set_value(ts_bytes);

    // --- 全部装进自定义消息并序列化 ---
    let mut boxed = AnyThing::new();
    boxed.set_any(any);
    boxed.set_duration(dur);
    boxed.set_object(obj);
    boxed.set_string_value(sv);
    boxed.set_int32_value(iv);
    boxed.set_nothing(Empty::new());
    boxed.set_mask(mask);

    let bytes = boxed.serialize().unwrap();
    let back = AnyThing::parse(&bytes).unwrap();
    println!("\nAnyThing wire = {} bytes", bytes.len());
    println!("  any.type_url  = {}", back.any().type_url());
    println!("  any.value     = {} bytes（内容需按 type_url 自己解码）", back.any().value().as_view().len());
    println!("  duration      = {}s", back.duration().seconds());
    println!("  object.fields = {}", back.object().fields().len());
    println!(
        "  object[\"name\"] = {}",
        back.object()
            .fields()
            .get("name")
            .unwrap()
            .string_value()
    );
    println!("  string_value  = {}", back.string_value().value());
    println!("  int32_value   = {}", back.int32_value().value());
    println!("  mask.paths    = {:?}", paths_of(back.mask()));
    println!("  nothing set?  = {}", back.nothing_opt().is_some());

    println!("\ntext format:\n{back:?}");
}

fn paths_of(mask: protobuf_well_known_types::FieldMaskView<'_>) -> Vec<String> {
    mask.paths()
        .iter()
        .map(|p| p.to_cow_lossy().into_owned())
        .collect()
}
