//! 场景 6：view / mut 零拷贝视图。
//!
//! 官方 API 为每个 message 生成三个代理类型：owns 的 `Msg`、只读的 `MsgView<'msg>`、
//! 可写的 `MsgMut<'msg>`。view/mut 都是借用，不拷贝数据，生命周期绑定在 owner 上。
//!
//! 运行：`cargo run --example 06_views`
//!
//! `as_view()` / `as_mut()` 是生成代码里的固有方法，不需要引入任何 trait。
use protobuf_rust_examples::pb::person_pb::{Person, PersonView};

/// 只接受只读视图：没有所有权，也不发生拷贝
fn summarize(p: PersonView<'_>) -> String {
    let tags: Vec<String> = p
        .tags()
        .iter()
        .map(|t| t.to_cow_lossy().into_owned())
        .collect();
    let score_sum: i32 = p.scores().iter().map(|(_, v)| v).sum();
    format!("name={} tags={tags:?} score_sum={score_sum}", p.name())
}

fn main() {
    let mut p = Person::new();
    p.set_name("Alice");
    p.set_id(1);
    p.tags_mut().push("rust");
    p.tags_mut().push("proto");
    p.scores_mut().insert("math", 95);
    p.scores_mut().insert("art", 88);

    // 借用出只读视图
    let view = p.as_view();
    println!("summarize(&view) = {}", summarize(view));
    println!("view 上同样能调所有 getter：id={} tags.len={}", view.id(), view.tags().len());

    // 借用出可写视图：改动直接反映到 owner 上
    {
        let mut m = p.as_mut();
        m.set_id(100);
        m.tags_mut().push("via-mut-view");
        m.scores_mut().insert("music", 70);
    }
    println!(
        "通过 mut 视图修改后：id={} tags={:?}",
        p.id(),
        p.tags()
            .iter()
            .map(|t| t.to_cow_lossy().into_owned())
            .collect::<Vec<_>>()
    );

    // 复合字段也有独立的视图类型，可以脱离 owner 单独迭代
    let tags_view = p.tags();
    println!("RepeatedView: first={:?} last={:?}", tags_view.first(), tags_view.last());
    let scores_view = p.scores();
    println!(
        "MapView: len={} keys={:?}",
        scores_view.len(),
        scores_view.keys().map(|k| k.to_cow_lossy().into_owned()).collect::<Vec<_>>()
    );

    // 子消息视图（`PointView<'msg>` 就是 `View<'msg, Point>`）
    let mut home = protobuf_rust_examples::pb::person_pb::Point::new();
    home.set_x(1);
    p.set_home(home);
    let home_view = p.home();
    println!("home_view = ({}, {})", home_view.x(), home_view.y());
}
