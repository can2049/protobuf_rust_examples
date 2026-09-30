//! 场景 1：最小闭环 —— 构造消息 → 序列化成 wire bytes → 解析回来。
//!
//! 运行：`cargo run --example 01_hello_wire`
use protobuf::prelude::*;
use protobuf_rust_examples::pb::person_pb::{Color, Person, Point};

fn main() {
    let mut p = Person::new();
    p.set_name("Alice");
    p.set_id(42);
    p.set_fav_color(Color::Green);
    p.tags_mut().push("vip");

    let mut home = Point::new();
    home.set_x(3);
    home.set_y(4);
    p.set_home(home);

    let bytes = p.serialize().unwrap();
    println!("serialized_len() = {}", p.serialized_len());
    println!("wire bytes ({} bytes) = {bytes:?}", bytes.len());

    // Debug 打印的是 protobuf text format，但官方实现用的是**字段号**而不是字段名
    println!("text format:\n{p:?}");

    let parsed = Person::parse(&bytes).unwrap();
    println!("name      = {}", parsed.name());
    println!("id        = {}", parsed.id());
    println!("fav_color = {:?}", parsed.fav_color());
    println!("home      = ({}, {})", parsed.home().x(), parsed.home().y());

    assert_eq!(parsed.name().to_cow_lossy(), "Alice");
    assert_eq!(parsed.id(), 42);
    assert_eq!(parsed.serialize().unwrap(), bytes, "序列化应当稳定可复现");

    // 空消息序列化后是 0 字节（proto3 零值不上 wire）
    println!("Person::new() -> {} bytes", Person::new().serialize().unwrap().len());
}
