//! 场景 7：流式读写 + 未知字段保留。
//!
//! 官方运行时里没有 length-delimited 的读写辅助（没有 `write_length_delimited` 之类），
//! 常见做法就是自己写 varint 长度前缀做分帧，这里演示一遍。
//! 另外演示 proto3 的未知字段在 parse → serialize 往返中会被保留。
//!
//! 运行：`cargo run --example 07_stream_and_unknown`
use protobuf::prelude::*;
use protobuf_rust_examples::pb::log_pb::LogRecord;

fn write_varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn read_varint(buf: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    let mut shift = 0;
    loop {
        let byte = *buf.get(*pos)?;
        *pos += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
}

/// 把多条消息编码成 `[varint 长度][payload]...` 的帧流
fn encode_framed(msgs: &[LogRecord]) -> Vec<u8> {
    let mut out = Vec::new();
    for m in msgs {
        let payload = m.serialize().unwrap();
        write_varint(&mut out, payload.len() as u64);
        out.extend_from_slice(&payload);
    }
    out
}

fn decode_framed(buf: &[u8]) -> Vec<LogRecord> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < buf.len() {
        let Some(len) = read_varint(buf, &mut pos) else {
            break;
        };
        let end = pos + len as usize;
        if end > buf.len() {
            // 帧不完整：丢掉尾巴，返回已经解出的完整帧
            break;
        }
        out.push(LogRecord::parse(&buf[pos..end]).unwrap());
        pos = end;
    }
    out
}

fn main() {
    let logs: Vec<LogRecord> = (0..3)
        .map(|i| {
            let mut l = LogRecord::new();
            l.set_ts_millis(1_700_000_000_000 + i);
            l.set_level(if i == 2 { "WARN" } else { "INFO" });
            l.set_message(format!("event #{i}"));
            l
        })
        .collect();

    let stream = encode_framed(&logs);
    println!("3 条日志 -> {} bytes 的帧流", stream.len());
    println!("帧流字节 = {stream:?}");

    let decoded = decode_framed(&stream);
    println!("解出 {} 条：", decoded.len());
    for l in &decoded {
        println!("  ts={} level={} msg={}", l.ts_millis(), l.level(), l.message());
    }
    assert_eq!(decoded.len(), logs.len());

    println!("\n--- 未知字段保留 ---");
    let mut raw = logs[0].serialize().unwrap();
    let len_before = raw.len();
    // 追加一个当前 schema 里不存在的字段 99（varint 类型）：
    // tag = (99 << 3) | 0 = 792 -> varint 编码为 [0x98, 0x06]
    raw.extend_from_slice(&[0x98, 0x06, 0x2a]);

    let parsed = LogRecord::parse(&raw).unwrap();
    let reserialized = parsed.serialize().unwrap();
    println!("原始 {len_before} bytes，加未知字段后 {} bytes", raw.len());
    println!("重新序列化 {} bytes，未知字段被保留：{}", reserialized.len(), reserialized == raw);
    println!("（parse 后能读到的已知字段不受影响：msg={}）", parsed.message());

    // 截断的帧流不应 panic，而是尽力解析出前面的完整帧
    let truncated = &stream[..stream.len() - 2];
    println!("\n截断帧流解出 {} 条（最后一条不完整被丢弃）", decode_framed(truncated).len());
}
