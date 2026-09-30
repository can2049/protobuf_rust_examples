#!/usr/bin/env bash
# 把官方 protobuf Rust 依赖升级到最新发布版，并同步 protoc 版本。
#
# 背景：官方约定 Rust protobuf crate 版本 x.y.z 必须配 protoc y.z。
# 本仓库把版本号写在 4 个地方，本脚本一次性改写它们：
#   Cargo.toml（3 条 pin） / build.rs（PROTOC_VERSION）
#   scripts/install-protoc.sh（PROTOC_VERSION、CRATE_VERSION） / README.md
#
# 用法：
#   bash scripts/update-protobuf.sh --check          # 只报告，不改文件、不下载（CI 可用）
#   bash scripts/update-protobuf.sh                  # 升级 + 拉 protoc + 编译 + 跑全部 example
#
# 需要人工处理的场景见 README「如何升级 protobuf 依赖版本」。
#
# 书写约定（为让替换安全）：提到 crate 版本时一律写 "4.36.2-release" 这种带后缀的完整形式，
# 不要写裸的 "4.36.2" 或 "4.36"；protoc 版本可以裸写，脚本会把旧的 protoc 版本串整体换掉。
set -euo pipefail

CRATE_PROTOBUF="protobuf"
CRATE_CODEGEN="protobuf-codegen"
CRATE_WKT="protobuf-well-known-types"
CRATE_RUNTIME="google-protobuf"

TARGET_FILES=(Cargo.toml build.rs scripts/install-protoc.sh README.md)

CHECK_ONLY=0
FORCE=0
SKIP_ASSET_CHECK=0
INDEX_BASE="${PROTOBUF_INDEX_BASE:-}"
DRIFT=()

info() { echo "[update] $*"; }
warn() { echo "[警告] $*" >&2; }
die() { echo "[错误] $*" >&2; exit 1; }

usage() {
  cat <<'EOF'
用法: bash scripts/update-protobuf.sh [选项]

  --check             只报告（含版本号漂移检测），不改文件、不下载；检测到漂移时退出码为 1
  --force             即便目标文件有未提交改动也继续改写
  --skip-asset-check  跳过 GitHub 上 protoc 预编译资产的可用性校验
  --index-base URL    crates.io sparse 索引基址（也可用环境变量 PROTOBUF_INDEX_BASE）
  -h, --help          显示本帮助
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check) CHECK_ONLY=1 ;;
    --force) FORCE=1 ;;
    --skip-asset-check) SKIP_ASSET_CHECK=1 ;;
    --index-base) INDEX_BASE="${2:-}"; shift ;;
    -h | --help) usage; exit 0 ;;
    *)
      echo "未知参数: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# ---------------------------------------------------------------- 索引

# 优先用 --index-base / PROTOBUF_INDEX_BASE，其次从 cargo 配置里取（本机走的是镜像源），
# 最后退化到 crates.io 官方索引。
detect_index_base() {
  if [[ -n "$INDEX_BASE" ]]; then
    echo "${INDEX_BASE%/}/"
    return
  fi
  local cfg url
  for cfg in "${CARGO_HOME:-$HOME/.cargo}/config.toml" "${CARGO_HOME:-$HOME/.cargo}/config"; do
    [[ -f "$cfg" ]] || continue
    # 只看未被注释的行（配置里常留被注释的镜像块）
    url="$(grep -v '^[[:space:]]*#' "$cfg" | grep -oE 'sparse\+https?://[^"[:space:]]+' | head -1 | sed 's|^sparse+||')"
    if [[ -z "$url" ]]; then
      url="$(grep -v '^[[:space:]]*#' "$cfg" |
        sed -nE 's/^[[:space:]]*registry[[:space:]]*=[[:space:]]*"(https?:\/\/[^"]+)".*/\1/p' | head -1)"
    fi
    if [[ -n "$url" ]]; then
      echo "${url%/}/"
      return
    fi
  done
  echo "https://index.crates.io/"
}

index_versions() { # $1 = crate 名，输出索引里的全部版本
  local crate="$1" body
  if ! body="$(curl -fsSL "${INDEX_BASE_RESOLVED}${crate:0:2}/${crate:2:2}/${crate}")"; then
    die "无法从索引拉取 ${crate}（${INDEX_BASE_RESOLVED}）；可用 --index-base 指定其它索引源"
  fi
  printf '%s' "$body" | grep -o '"vers":"[^"]*"' | sed -e 's/^"vers":"//' -e 's/"$//' || true
}

# 只保留正式发布版（排除 -rc.* 与 yanked 无法在此判定，由后续 cargo 兜底）
stable_only() { grep -E '^[0-9]+\.[0-9]+\.[0-9]+-release$' || true; }

latest_stable() { # $1 = crate 名
  index_versions "$1" | stable_only | sort -t. -k1,1n -k2,2n -k3,3n | tail -1
}

# 官方规则：crate 4.36.2-release -> protoc 36.2
# 与 protobuf-codegen 内部 expected_protoc_version() 一致：去掉首段与 -后缀
protoc_version_of() { echo "${1%%-*}" | cut -d. -f2,3; }

match_by_protoc() { # $1 = crate 名, $2 = 目标 protoc 版本；返回推导结果相同的版本
  local crate="$1" want="$2"
  index_versions "$crate" | stable_only | awk -v want="$want" '{
      core = $0; sub(/-.*$/, "", core);
      if (split(core, p, ".") == 3 && p[2] "." p[3] == want) print $0;
    }' | sort -t. -k1,1n -k2,2n -k3,3n | tail -1
}

version_gt() { [[ "$1" != "$2" && "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" == "$1" ]]; }

# ---------------------------------------------------------------- 读写现状

read_pin() { # $1 = crate 名
  sed -nE "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*\"=?([^\"]+)\".*/\1/p" Cargo.toml | head -1
}

# 扫描硬编码位置，找出与 Cargo.toml 推导值不一致的地方
collect_drift() {
  DRIFT=()
  local found v
  found="$(sed -nE 's/.*PROTOC_VERSION: &str = "([0-9]+\.[0-9]+)".*/\1/p' build.rs | head -1)"
  [[ "$found" == "$CUR_PROTOC" ]] ||
    DRIFT+=("build.rs: PROTOC_VERSION = ${found:-未找到}，应为 $CUR_PROTOC")

  found="$(sed -nE 's/^PROTOC_VERSION="([^"]+)".*/\1/p' scripts/install-protoc.sh | head -1)"
  [[ "$found" == "$CUR_PROTOC" ]] ||
    DRIFT+=("scripts/install-protoc.sh: PROTOC_VERSION = ${found:-未找到}，应为 $CUR_PROTOC")

  found="$(sed -nE 's/^CRATE_VERSION="([^"]+)".*/\1/p' scripts/install-protoc.sh | head -1)"
  [[ "$found" == "$CUR_PROTOBUF" ]] ||
    DRIFT+=("scripts/install-protoc.sh: CRATE_VERSION = ${found:-未找到}，应为 $CUR_PROTOBUF")

  while read -r v; do
    [[ -n "$v" ]] || continue
    [[ "$v" == "$CUR_PROTOBUF" || "$v" == "$CUR_WKT" ]] ||
      DRIFT+=("README.md: 出现其它版本字样 $v（当前 pin: $CUR_PROTOBUF / $CUR_WKT）")
  done < <(grep -oE '[0-9]+\.[0-9]+\.[0-9]+-release' README.md | sort -u || true)

  while read -r v; do
    [[ -n "$v" ]] || continue
    [[ "$v" == "$CUR_PROTOC" ]] ||
      DRIFT+=("README.md: 出现 protoc $v（应为 $CUR_PROTOC）")
  done < <(grep -oE 'protoc[- ]?[0-9]+\.[0-9]+' README.md | grep -oE '[0-9]+\.[0-9]+' | sort -u || true)
}

check_asset() { # $1 = protoc 版本
  local version="$1" arch url
  case "$(uname -m)" in
    x86_64 | amd64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *)
      warn "未知架构 $(uname -m)，跳过 protoc 资产校验"
      return 0
      ;;
  esac
  url="https://github.com/protocolbuffers/protobuf/releases/download/v${version}/protoc-${version}-linux-${arch}.zip"
  if curl -fsSIL -o /dev/null "$url"; then
    info "protoc 资产可用: ${url}"
  else
    die "找不到 protoc ${version} 的预编译资产（${url}）。
      可用 --skip-asset-check 跳过校验。注意：官方 RC / 预发布版本不提供 zip，需要自行编译 protoc。"
  fi
}

check_worktree() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 0
  local dirty=() f
  for f in "${TARGET_FILES[@]}"; do
    if ! git diff --quiet -- "$f" 2>/dev/null; then dirty+=("$f"); fi
  done
  if [[ ${#dirty[@]} -gt 0 ]]; then
    warn "以下文件有未提交改动，改写会覆盖它们：${dirty[*]}"
    [[ $FORCE -eq 1 ]] || die "请先提交或 stash，或加 --force 继续"
  fi
}

escape_dots() { printf '%s' "${1//./\\.}"; }

# 两阶段替换：旧值先换成占位符，再统一写新值，避免旧/新版本号互为子串时连锁替换
apply_replacements() {
  local pb_pat wkt_pat protoc_pat f
  pb_pat="$(escape_dots "$CUR_PROTOBUF")"
  wkt_pat="$(escape_dots "$CUR_WKT")"
  protoc_pat="$(escape_dots "$CUR_PROTOC")"
  for f in "${TARGET_FILES[@]}"; do
    [[ -f "$f" ]] || continue
    sed -i \
      -e "s|${pb_pat}|@@PROTOBUF_CRATE_VERSION@@|g" \
      -e "s|${wkt_pat}|@@WKT_CRATE_VERSION@@|g" \
      -e "s|${protoc_pat}|@@PROTOC_VERSION@@|g" \
      -e "s|@@PROTOBUF_CRATE_VERSION@@|${TARGET_PROTOBUF}|g" \
      -e "s|@@WKT_CRATE_VERSION@@|${TARGET_WKT}|g" \
      -e "s|@@PROTOC_VERSION@@|${TARGET_PROTOC}|g" \
      "$f"
  done
}

print_changed_files() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 0
  git --no-pager diff --stat -- "${TARGET_FILES[@]}" || true
  git status --short -- "${TARGET_FILES[@]}" || true
}

# ---------------------------------------------------------------- 主流程

INDEX_BASE_RESOLVED="$(detect_index_base)"
info "索引源: ${INDEX_BASE_RESOLVED}"

info "查找最新发布版 ..."
TARGET_PROTOBUF="$(latest_stable "$CRATE_PROTOBUF")"
[[ -n "$TARGET_PROTOBUF" ]] || die "索引里找不到 ${CRATE_PROTOBUF} 的正式版（*-release）"
TARGET_PROTOC="$(protoc_version_of "$TARGET_PROTOBUF")"
TARGET_CODEGEN="$(match_by_protoc "$CRATE_CODEGEN" "$TARGET_PROTOC")"
TARGET_WKT="$(match_by_protoc "$CRATE_WKT" "$TARGET_PROTOC")"
[[ -n "$TARGET_CODEGEN" ]] || die "索引里没有与 protoc ${TARGET_PROTOC} 匹配的 ${CRATE_CODEGEN}"
[[ -n "$TARGET_WKT" ]] || die "索引里没有与 protoc ${TARGET_PROTOC} 匹配的 ${CRATE_WKT}"

CUR_PROTOBUF="$(read_pin "$CRATE_PROTOBUF")"
CUR_CODEGEN="$(read_pin "$CRATE_CODEGEN")"
CUR_WKT="$(read_pin "$CRATE_WKT")"
[[ -n "$CUR_PROTOBUF" && -n "$CUR_CODEGEN" && -n "$CUR_WKT" ]] ||
  die "没能从 Cargo.toml 读出 ${CRATE_PROTOBUF} / ${CRATE_CODEGEN} / ${CRATE_WKT} 的 pin"
CUR_PROTOC="$(protoc_version_of "$CUR_PROTOBUF")"

info "当前 pin : ${CUR_PROTOBUF} / ${CUR_CODEGEN} / ${CUR_WKT} / protoc ${CUR_PROTOC}"
info "最新发布 : ${TARGET_PROTOBUF} / ${TARGET_CODEGEN} / ${TARGET_WKT} / protoc ${TARGET_PROTOC}"

LATEST_RUNTIME="$(latest_stable "$CRATE_RUNTIME")"
if [[ -n "$LATEST_RUNTIME" ]]; then
  runtime_protoc="$(protoc_version_of "$LATEST_RUNTIME")"
  if version_gt "$runtime_protoc" "$TARGET_PROTOC"; then
    warn "${CRATE_RUNTIME} 最新为 ${LATEST_RUNTIME}（protoc ${runtime_protoc}），比门面 ${CRATE_PROTOBUF} 的 protoc ${TARGET_PROTOC} 更新：可能内部运行时已发布但门面尚未同步，可稍后再试"
  fi
fi

collect_drift
if [[ ${#DRIFT[@]} -gt 0 ]]; then
  warn "检测到版本号不一致（漂移）："
  printf '  - %s\n' "${DRIFT[@]}" >&2
fi

UP_TO_DATE=0
if [[ "$CUR_PROTOBUF" == "$TARGET_PROTOBUF" && "$CUR_CODEGEN" == "$TARGET_CODEGEN" && "$CUR_WKT" == "$TARGET_WKT" ]]; then
  UP_TO_DATE=1
fi

if [[ $CHECK_ONLY -eq 1 ]]; then
  if [[ $UP_TO_DATE -eq 1 ]]; then
    info "已是最新：${CUR_PROTOBUF} / protoc ${CUR_PROTOC}（未改动任何文件）"
  else
    info "可升级到：${TARGET_PROTOBUF} / protoc ${TARGET_PROTOC}（去掉 --check 即为正式升级，未改动任何文件）"
  fi
  [[ ${#DRIFT[@]} -eq 0 ]] || exit 1
  exit 0
fi

if [[ $UP_TO_DATE -eq 1 ]]; then
  info "已是最新：${CUR_PROTOBUF} / protoc ${CUR_PROTOC}，无需升级"
  [[ ${#DRIFT[@]} -eq 0 ]] || die "但存在版本号漂移，请手工修正上面列出的位置"
  exit 0
fi

if [[ "$CUR_PROTOBUF" == "$CUR_WKT" && "$TARGET_PROTOBUF" != "$TARGET_WKT" ]]; then
  die "Cargo.toml 里 ${CRATE_PROTOBUF} 与 ${CRATE_WKT} 当前版本号相同，但目标版本不同：替换存在歧义，请手工改一次"
fi

[[ $SKIP_ASSET_CHECK -eq 1 ]] || check_asset "$TARGET_PROTOC"
check_worktree

apply_replacements
info "已改写：${TARGET_FILES[*]}"
print_changed_files

info "拉取 protoc ${TARGET_PROTOC}"
bash scripts/install-protoc.sh "$TARGET_PROTOC"

info "编译 examples（首次需要编译 upb C 内核，可能较慢）"
if ! cargo build --examples; then
  warn "编译失败：官方 API / 生成代码可能已变更，需要人工修改 src/ 或 examples/"
  warn "回滚：git checkout -- ${TARGET_FILES[*]} && cargo update"
  exit 1
fi

info "运行全部 example"
failed=()
for f in examples/*.rs; do
  name="$(basename "$f" .rs)"
  if cargo run --quiet --example "$name" >/dev/null 2>&1; then
    echo "  ok    ${name}"
  else
    echo "  FAIL  ${name}" >&2
    failed+=("$name")
  fi
done

echo
if [[ ${#failed[@]} -eq 0 ]]; then
  info "升级完成：${CUR_PROTOBUF} -> ${TARGET_PROTOBUF}，protoc ${TARGET_PROTOC}，全部 example 通过"
  info "建议复核上面的改动后提交（Cargo.lock 也会随之更新）"
else
  warn "${#failed[@]} 个 example 失败：${failed[*]}"
  warn "多半是官方 API / 生成代码变更，需要人工修改对应 example"
  warn "回滚：git checkout -- ${TARGET_FILES[*]} && cargo update"
  exit 1
fi
