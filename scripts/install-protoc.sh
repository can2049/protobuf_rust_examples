#!/usr/bin/env bash
# 下载与 protobuf crate 版本精确匹配的 protoc。
#
# 官方 Rust 实现要求 protoc 版本与运行时版本精确对齐：
#   protobuf crate 4.36.2-release  <->  protoc 36.2
# 即 crate 版本去掉首段与 -release 后缀就是所需 protoc 版本。
# 版本不匹配时 protobuf-codegen 会在构建期直接 panic。
#
# 用法：
#   bash scripts/install-protoc.sh            # 用下面 PROTOC_VERSION 常量
#   bash scripts/install-protoc.sh 36.2       # 显式指定版本（升级脚本用这种形式）
#
# 注意：crate 版本一律写带 -release 后缀的完整形式，便于 scripts/update-protobuf.sh 做替换。
set -euo pipefail

PROTOC_VERSION="36.2"          # 与 Cargo.toml 里的 protobuf = "=4.36.2-release" 对应
CRATE_VERSION="4.36.2-release"

if [[ $# -ge 1 && -n "${1:-}" ]]; then
  PROTOC_VERSION="$1"
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest_dir="${repo_root}/tools/protoc-${PROTOC_VERSION}"

case "$(uname -m)" in
  x86_64 | amd64) protoc_arch="x86_64" ;;
  aarch64 | arm64) protoc_arch="aarch64" ;;
  *)
    echo "不支持的架构: $(uname -m)" >&2
    exit 1
    ;;
esac

asset="protoc-${PROTOC_VERSION}-linux-${protoc_arch}.zip"
url="https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${asset}"

echo "目标 protoc 版本: ${PROTOC_VERSION} (对应 protobuf crate ${CRATE_VERSION})"
echo "安装目录: ${dest_dir}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

echo "下载 ${url}"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "${tmp_dir}/${asset}" "${url}"
elif command -v wget >/dev/null 2>&1; then
  wget -q -O "${tmp_dir}/${asset}" "${url}"
else
  echo "需要 curl 或 wget" >&2
  exit 1
fi

rm -rf "${dest_dir}"
mkdir -p "${dest_dir}"
unzip -q "${tmp_dir}/${asset}" -d "${dest_dir}"

protoc_bin="${dest_dir}/bin/protoc"
chmod +x "${protoc_bin}"

actual="$("${protoc_bin}" --version)"
echo "安装完成: ${actual}"
if [[ "${actual}" != "libprotoc ${PROTOC_VERSION}" ]]; then
  echo "版本不符，期望 libprotoc ${PROTOC_VERSION}，实际 ${actual}" >&2
  exit 1
fi
