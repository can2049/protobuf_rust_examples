//! 场景 4：oneof 分支切换 + enum（含未知枚举值）。
//!
//! 运行：`cargo run --example 04_oneof_enum`
use protobuf::prelude::*;
use protobuf_rust_examples::pb::person_pb::{person, Color, Person};

fn main() {
    let mut p = Person::new();
    println!("初始 contact_case = {:?}", p.contact_case());

    p.set_phone("138-0000-0000");
    println!("设置 phone 之后：");
    println!("  contact_case = {:?}", p.contact_case());
    println!("  contact()    = {:?}", p.contact());
    println!("  phone_opt    = {:?}", p.phone_opt());
    println!("  address_opt  = {:?}", p.address_opt());

    // 设置另一个分支会自动清空前一个分支
    p.set_address("深圳市南山区");
    println!("改设 address 之后：");
    println!("  contact_case = {:?}", p.contact_case());
    println!("  phone_opt    = {:?}（已被清空）", p.phone_opt());

    match p.contact() {
        person::ContactOneof::Phone(s) => println!("  match -> Phone({s})"),
        person::ContactOneof::Address(s) => println!("  match -> Address({s})"),
        _ => println!("  match -> 未设置"),
    }

    // oneof 也参与序列化：只有当前分支上 wire
    println!("wire = {:?}", p.serialize().unwrap());

    println!("\n--- enum ---");
    let mut e = Person::new();
    e.set_fav_color(Color::Blue);
    println!("已知值 Color::Blue -> {:?}", e.fav_color());
    println!("枚举常量：{:?} {:?} {:?}", Color::Unspecified, Color::Red, Color::Green);

    // 未知枚举值：proto3 会原样保留（field 5 = fav_color，varint 99）
    let unknown_bytes = [40u8, 99u8];
    let unknown = Person::parse(&unknown_bytes).unwrap();
    println!("未知枚举值 99 解析后 = {:?}", unknown.fav_color());
    println!(
        "原样序列化回去 = {:?}（与输入一致即未知值未被丢弃）",
        unknown.serialize().unwrap()
    );

    println!("\n--- oneof 的 wire 编码 ---");
    let mut a = Person::new();
    a.set_name("x");
    let mut b = Person::new();
    b.set_name("x");
    b.set_phone("p");
    let a_bytes = a.serialize().unwrap();
    let b_bytes = b.serialize().unwrap();
    println!("只有 name     -> {} bytes {a_bytes:?}", a_bytes.len());
    println!("name + phone -> {} bytes {b_bytes:?}", b_bytes.len());
}
