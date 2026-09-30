//! 场景 3：嵌套 message、repeated、map，以及跨 proto 文件的类型引用。
//!
//! 运行：`cargo run --example 03_composite`
use protobuf::prelude::*;
use protobuf_rust_examples::pb::catalog_pb::Team;
use protobuf_rust_examples::pb::person_pb::{person, Person, Point};

fn main() {
    let mut p = Person::new();
    p.set_name("Alice");
    p.set_id(1);

    // 嵌套 message：proto 里写的 `Person.Address` 生成在 `person` 子模块下
    let mut addr = person::Address::new();
    addr.set_city("深圳");
    addr.set_street("科技园路 1 号");
    p.set_work(addr);
    println!("work.city = {}", p.work().city());
    println!("未设置的子消息也能直接读：p.home().x() = {}", p.home().x());

    // 子消息字段：set_xxx 接受实现了 IntoProxied 的值
    let home = {
        let mut h = Point::new();
        h.set_x(3);
        h.set_y(4);
        h
    };
    p.set_home(home);

    println!("\n--- repeated ---");
    p.tags_mut().push("rust");
    p.tags_mut().push("protobuf");
    p.tags_mut().set(1, "grpc");
    println!("tags = {:?}", str_list(p.tags()));
    println!("len = {}, first = {:?}", p.tags().len(), p.tags().first().map(|t| t.to_cow_lossy().into_owned()));
    println!("get(0) = {:?}", p.tags().get(0).map(|t| t.to_cow_lossy().into_owned()));

    println!("\n--- map ---");
    p.scores_mut().insert("math", 95);
    p.scores_mut().insert("art", 88);
    println!("scores = {:?}", map_entries(p.scores()));
    println!("contains_key(\"math\") = {}", p.scores().contains_key("math"));
    println!("get(\"math\") = {:?}", p.scores().get("math"));
    p.scores_mut().remove("art");
    println!("remove 之后 scores = {:?}", map_entries(p.scores()));

    println!("\n--- 跨 proto 文件引用（catalog.proto 里定义 Team，引用 person.proto 的 Person）---");
    let mut team = Team::new();
    team.set_name("core");
    team.members_mut().push(p);
    let mut second = Person::new();
    second.set_name("Bob");
    second.set_id(2);
    team.members_mut().push(second);
    team.roster_mut().insert("alice", {
        let mut a = Person::new();
        a.set_name("Alice");
        a.set_id(1);
        a
    });

    let bytes = team.serialize().unwrap();
    let back = Team::parse(&bytes).unwrap();
    println!("Team wire = {} bytes", bytes.len());
    println!("name = {}, members = {}", back.name(), back.members().len());
    for m in back.members().iter() {
        println!("  member: {} ({})", m.name(), m.id());
    }
    println!("roster keys = {:?}", back.roster().keys().map(|k| k.to_cow_lossy().into_owned()).collect::<Vec<_>>());
}

fn str_list(v: protobuf::RepeatedView<'_, protobuf::ProtoString>) -> Vec<String> {
    v.iter().map(|t| t.to_cow_lossy().into_owned()).collect()
}

fn map_entries(v: protobuf::MapView<'_, protobuf::ProtoString, i32>) -> Vec<(String, i32)> {
    v.iter().map(|(k, val)| (k.to_cow_lossy().into_owned(), val)).collect()
}
