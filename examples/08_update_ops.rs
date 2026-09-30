//! 场景 8：消息的复用与合并 —— Clear / ClearAndParse / MergeFrom / CopyFrom / TakeFrom。
//!
//! 这些 trait 由 prelude 引入，`proto!`/消息类型上可以直接调用。
//!
//! 运行：`cargo run --example 08_update_ops`
use protobuf::prelude::*;
use protobuf_rust_examples::pb::person_pb::{Person, Point};

fn dump(tag: &str, p: &Person) {
    println!(
        "{tag}: name={:?} id={} tags={:?} home=({}, {})",
        p.name().to_cow_lossy(),
        p.id(),
        p.tags()
            .iter()
            .map(|t| t.to_cow_lossy().into_owned())
            .collect::<Vec<_>>(),
        p.home().x(),
        p.home().y()
    );
}

fn main() {
    let mut a = Person::new();
    a.set_name("Alice");
    a.set_id(1);
    a.tags_mut().push("a");
    let mut home_a = Point::new();
    home_a.set_x(1);
    a.set_home(home_a);

    let mut b = Person::new();
    b.set_id(2);
    b.tags_mut().push("b");
    let mut home_b = Point::new();
    home_b.set_y(9);
    b.set_home(home_b);

    dump("a", &a);
    dump("b", &b);

    // MergeFrom：标量覆盖（id: 1 -> 2）、repeated 追加、子消息递归合并（home.x 保留、home.y 补上）
    let mut merged = a.clone();
    merged.merge_from(&b);
    dump("merge_from(b)", &merged);

    // CopyFrom：内容复制，src 不变
    let mut copied = Person::new();
    copied.copy_from(&a);
    dump("copy_from(a)", &copied);
    dump("a 仍然", &a);

    // TakeFrom：把内容搬走，src 回到默认状态
    let mut taken = Person::new();
    taken.take_from(&mut copied);
    dump("take_from(copied)", &taken);
    dump("copied 被搬空后", &copied);

    // ClearAndParse：复用同一个实例解析新数据，省去反复分配
    let bytes = a.serialize().unwrap();
    let mut reused = Person::new();
    reused.set_name("临时值会被清掉");
    reused.set_id(999);
    reused.clear_and_parse(&bytes).unwrap();
    dump("clear_and_parse(a 的字节)", &reused);

    // Clear：清空内容但保留 arena 容量（同样的对象可继续复用）
    reused.clear();
    dump("clear() 之后", &reused);
    println!("clear 之后 serialize() = {:?}", reused.serialize().unwrap());
}
