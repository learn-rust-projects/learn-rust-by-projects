#!/usr/bin/env bash
set -euo pipefail

# 颜色输出
bold() { echo -e "\033[1m$*\033[0m"; }
info() { echo -e "\033[36m[INFO]\033[0m $*"; }
ok() { echo -e "\033[32m[OK]\033[0m $*"; }
fail() { echo -e "\033[31m[FAIL]\033[0m $*" >&2; exit 1; }

# 打印步骤标题
print_step() {
    echo -e "\033[33m▶ $*\033[0m"
}

# 运行程序初始化
print_step "运行初始化命令"
PROGRAM="$1"
info "执行命令: $PROGRAM init"
"$PROGRAM" init
ok "初始化完成"

# 检查 .git 目录结构
print_step "检查 .git 目录结构"
test -d .git              && ok "✓ .git 目录存在"       || fail "✗ .git 目录缺失"
test -d .git/objects      && ok "✓ .git/objects 存在"         || fail "✗ .git/objects 缺失"
test -d .git/refs         && ok "✓ .git/refs 存在"            || fail "✗ .git/refs 缺失"
test -f .git/HEAD         && ok "✓ .git/HEAD 存在"            || fail "✗ .git/HEAD 缺失"

# 检查文件格式
print_step "检查文件格式规范"
LAST_BYTE=$(tail -c1 .git/HEAD| od -c)

if [[ "$LAST_BYTE" == *$'\n'* ]]; then
    ok "✓ 文件以 Unix 换行 (LF) 结尾"
else
    fail "✗ 文件没有换行符结尾"
fi

# 检查引用内容
print_step "验证 HEAD 引用内容"
HEAD_CONTENT=$(cat .git/HEAD)
# 检查引用是否合法
if [[ "$HEAD_CONTENT" == "ref: refs/heads/main" || "$HEAD_CONTENT" == "ref: refs/heads/master" ]]; then
    ok "✓ .git/HEAD 包含有效引用: $HEAD_CONTENT"
else
    fail "✗ .git/HEAD 包含无效引用: $HEAD_CONTENT"
fi

# 显示测试结果
bold "\n✅ Git 初始化测试全部通过!"