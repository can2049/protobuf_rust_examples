# protobuf_rust_examples

演示 **protobuf 官方 Rust 支持**（Google 的 `google-protobuf` 运行时 + `protoc --rust_out` 代码生成器）的常见使用场景。

参考资料：

- <https://github.com/protocolbuffers/protobuf>
- <http://protobuf.dev/>

> 注意区分：官方实现是 Protobuf 团队从 2024 年起开发的**全新实现**（门面 crate `protobuf` + 运行时 `google-protobuf`，底层是 upb / C++ 内核），
> 与社区维护多年的 `protobuf` 3.x（stepancheg/rust-protobuf，纯 Rust）**API 完全不同**。本仓库用的是官方实现。

## 依赖与版本对应关系

| 组件 | 版本 | 说明 |
| --- | --- | --- |
| `protobuf` | `=4.36.2-release` | 门面 crate，实际内容来自 `google-protobuf` |
| `google-protobuf` | `0.36.2-release` | 运行时本体（默认编译 upb C 内核） |
| `protobuf-codegen` | `=4.36.2-release` | build 依赖，负责调 protoc 生成代码 |
| `protobuf-well-known-types` | `=0.36.2-release` | WKT（Timestamp / Struct / Any …）的预生成绑定 |
| **`protoc`** | **`36.2`** | **必须精确匹配** |

对应规则（官方约定）：crate 版本去掉首段与 `-release` 后缀就是所需的 protoc 版本，
例如 `4.36.2-release` ↔ `protoc 36.2`。版本不符时 `protobuf-codegen` 会在构建期直接 panic。

版本号如何升级见文末「如何升级 protobuf 依赖版本」。

## 前置：安装匹配版本的 protoc

系统里自带的 protoc（比如 3.13）**不能用**，它没有内置 Rust 生成器。

```bash
bash scripts/install-protoc.sh   # 下载 protoc 36.2 到 tools/protoc-36.2/
```

脚本会按架构选择 `protoc-36.2-linux-{x86_64,aarch64}.zip` 并校验版本，也支持显式传版本
（`bash scripts/install-protoc.sh 36.2`）。
`build.rs` 里定位 protoc 的优先级：环境变量 `PROTOC` → `tools/protoc-36.2/bin/protoc` → `PATH` 上的 `protoc`。

## 快速开始

```bash
cargo build                      # 首次会编译 upb C 内核，稍慢
cargo run --example 01_hello_wire
```

## 目录结构

```
Cargo.toml          # 依赖精确 pin（-release 后缀不能省）
build.rs            # 定位 protoc + 调 protobuf_codegen::CodeGen 生成代码
scripts/            # protoc 安装脚本 + 依赖升级脚本
protos/             # .proto 定义
src/lib.rs          # include! 生成的代码，暴露为 pb::*
examples/           # 每个场景一个可运行示例
```

### 生成代码的模块命名约束（重要）

`protoc --rust_out` 生成的代码内部会用 `super::super::<proto 路径下划线化>::...` 做自引用，
所以 `include!` 所在的模块**必须**按 proto 路径命名，并且所有 proto 模块必须是**同一父模块下的兄弟模块**：

```
protos/person.proto   ->  pub mod protos_person_proto   // 名字不能改
```

见 `src/lib.rs`：全部放在 `pb` 下，再用 `pub use ... as person_pb` 起别名方便示例阅读。
proto 之间 import 要写相对 crate 根的全路径，例如 `import "protos/person.proto";`。

## 运行示例

| 命令 | 场景 |
| --- | --- |
| `cargo run --example 01_hello_wire` | 最小闭环：构造 → `serialize()` → `parse()` → Debug 输出 |
| `cargo run --example 02_scalars_presence` | 全部标量类型；proto3 零值不上 wire；`optional` 的显式 presence |
| `cargo run --example 03_composite` | 嵌套 message、repeated、map、跨 proto 文件引用 |
| `cargo run --example 04_oneof_enum` | oneof 分支切换与 `*_case()`；enum 常量与未知枚举值 |
| `cargo run --example 05_well_known_types` | Timestamp / Duration / Struct / Wrappers / FieldMask / Empty / Any |
| `cargo run --example 06_views` | view / mut 零拷贝视图（`as_view()` / `as_mut()`） |
| `cargo run --example 07_stream_and_unknown` | 手写 varint 长度前缀做流式分帧；未知字段在往返中保留 |
| `cargo run --example 08_update_ops` | Clear / ClearAndParse / MergeFrom / CopyFrom / TakeFrom |

## 能力边界（基于当前 pin 的版本实测）

**已有**

- 二进制 wire 编解码：`serialize()` / `parse()` / `serialized_len()` / `clear_and_parse()`
- 全部字段类型：标量、嵌套 message、repeated、map、enum、proto3 `optional` presence、oneof
- 消息操作：`merge_from` / `copy_from` / `take_from` / `clear`
- 零拷贝视图：`MsgView<'msg>` / `MsgMut<'msg>`，以及 `RepeatedView` / `MapView`
- well-known types 绑定；未知字段保留

**尚不支持**（官方仍在开发中，标为 beta/experimental）

- **JSON 编解码**（没有 `protobuf-json-mapping` 的对应物）
- **text format 解析**：只有 `Debug` 能打印，且**输出的是字段号而不是字段名**（`1: "Alice"`）
- **反射 / `DynamicMessage` / 描述符遍历**
- **serde 集成**
- length-delimited 流式读写辅助（自己要写 varint 前缀，见 example 07）
- `Any` 的 pack / unpack 辅助（type_url + value 要自己拼，见 example 05）

如果你需要 JSON / 反射 / text-format 解析，应使用社区的 `protobuf` 3.x（rust-protobuf）；
注意两者 crate 同名但大版本不兼容，同一个 crate 里混用会很别扭（生成代码里写死了 `::protobuf::` 路径）。

## 常见坑

1. **`cargo add protobuf` 装错版本**：官方版本号带 `-release` 后缀（semver 预发布），
   直接 `cargo add protobuf` 会解析到社区的 `3.7.2`。必须写 `=4.36.2-release`。
2. **protoc 版本必须精确匹配**：`protoc --version` 必须是 `libprotoc 36.2`，否则构建失败。
3. **生成代码的模块名不能改**：见上文「模块命名约束」，改名会编译不过。
4. **首次构建较慢**：`google-protobuf` 默认编译 upb C 内核，需要可用的 C 编译器（`cc` / `clang`）。
5. **生成代码会刷一堆 warning**：已在 `src/lib.rs` 的 `pub mod pb` 上用 `#[allow(nonstandard_style, ...)]` 收敛。
6. **`Debug` 不是字段名**：`{msg:?}` 输出 text format 但只有字段号，调试时对着 `.proto` 数一下字段号。

## 如何升级 protobuf 依赖版本

版本号散落在四个地方（`Cargo.toml`、`build.rs`、`scripts/install-protoc.sh`、本文件），
**不要手改数字**，用脚本一次性升级：

```bash
bash scripts/update-protobuf.sh --check   # 只报告：当前 pin、最新发布版、版本号是否漂移
bash scripts/update-protobuf.sh           # 正式升级
```

脚本做的事：

1. 从 cargo 索引找 `protobuf` 最新的正式版；索引源会自动读本机 cargo 配置里的镜像，
   也可用 `--index-base` 或环境变量 `PROTOBUF_INDEX_BASE` 指定（比如 `https://index.crates.io/`）。
2. 按官方规则推导所需 protoc 版本（crate 版本去掉首段与 `-release` 后缀），
   再据此在同族 crate（`protobuf-codegen`、`protobuf-well-known-types`）里匹配**同一 protoc 版本**的正式版。
   这样能适应官方中途调整过的版本方案（`protobuf-well-known-types` 就在 4.35 → 0.36 时换过一次编号方式）。
3. 校验 GitHub 上对应 protoc 的预编译资产是否存在（`--skip-asset-check` 可跳过）。
4. 改写上述四个文件里的版本号，然后调用 `scripts/install-protoc.sh` 拉取新的 protoc。
5. `cargo build --examples` 并逐个运行 8 个 example，最后汇总成功/失败。

退出码：`--check` 在「已是最新/可升级」时为 0，检测到版本号漂移（硬编码位置与 `Cargo.toml` 推导值不一致）为 1；
正式升级时只要编译或任一 example 失败就为非 0，并给出回滚命令。

### 无法自动化、必须人工处理的

1. **官方 API / 生成代码可能变化**：setter 命名、`*_opt` 风格、生成模块命名（`protos_person_proto`）、
   `--rust_opt` 参数、kernel feature（`kernel=upb`）。脚本只会报编译/运行失败，不会替你改代码。
2. **版本方案可能再次变化**：若官方用上四段版本号（`protobuf-codegen` 内部 `assert_eq!(v.len(), 3)` 会直接 panic）
   或去掉 `-release` 后缀，需要更新 `scripts/update-protobuf.sh` 的过滤与推导规则。
3. **能力边界可能变化**：官方一旦支持 JSON / 反射 / text-format 解析，要回来更新上文「能力边界」并补示例。
4. **RC / 预发布**：官方 GitHub release 不提供 `-rc` 的 protoc 预编译 zip，想试 RC 得自行编译 protoc；
   升级脚本只处理 `-release`。
5. **protoc 资产命名或平台矩阵变化**：改 `scripts/install-protoc.sh` 里的 URL 模板与架构映射。
6. **需要 pin 的 crate 清单变化**：官方新增/移除配套 crate（例如将来独立的 JSON mapping crate）时，
   要同时改 `Cargo.toml` 和升级脚本里的 crate 列表。
7. **镜像源滞后**：本机默认走镜像，可能看不到刚发布的版本，必要时用 `--index-base https://index.crates.io/`。
8. 人工核对官方版本支持表 <https://protobuf.dev/support/version-support/>。

### 书写约定（让替换保持安全）

升级脚本靠字符串替换改版本号，所以新写文档/脚本时请遵守：

- 提到 crate 版本一律写完整形式 `4.36.2-release`，**不要**写裸的 `4.36.2` 或 `4.36`；
- protoc 版本可以裸写，但不要写入与当前 pin 不一致的 protoc 版本（`--check` 会报漂移）；
- 要举例说明版本规则时，用 `x.y.z` 这类占位符，别用真实版本号。

## 许可协议

本项目采用 [MIT](https://opensource.org/licenses/MIT) 协议，全文见 [LICENSE](LICENSE)。

本项目依赖的 protobuf 官方实现（`google-protobuf` 运行时、`protoc` 代码生成器）由 Google 以
BSD-3-Clause 授权，仅通过 Cargo 依赖使用，未修改其源码；`protos/` 下的示例 proto 文件为本项目自行编写。
