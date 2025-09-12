#!/usr/bin/env bash
set -euo pipefail

PROGRAM="$1"   # 传入你的 own-git 可执行程序
TEST_FILE="test.txt"

# 颜色输出函数
bold() { echo -e "\033[1m$*\033[0m"; }
info() { echo -e "\033[36m[INFO]\033[0m $*"; }
ok() { echo -e "\033[32m[OK]\033[0m $*"; }
fail() { echo -e "\033[31m[FAIL]\033[0m $*" >&2; exit 1; }

# 打印步骤标题
print_step() {
    echo -e "\033[33m▶ $*\033[0m"
}

# ===== 写入测试文件 =====
print_step "创建测试文件"
RAND_CONTENT=$(head /dev/urandom | tr -dc 'a-zA-Z0-9' | head -c 16)
echo "$RAND_CONTENT" > "$TEST_FILE"
info "生成的随机内容: $RAND_CONTENT"
ok "✓ 测试文件已创建"

# ===== 使用 git hash-object 获取标准 hash =====
print_step "计算文件哈希值"
EXPECTED_HASH=$("$PROGRAM" hash-object -w "$TEST_FILE")
info "计算得到的哈希: $EXPECTED_HASH"
ok "✓ 哈希计算完成"

# ===== 使用 own-git 读取 blob =====
print_step "验证文件内容读取"
OUTPUT=$("$PROGRAM" cat-file -p "$EXPECTED_HASH")
info "读取的内容: $OUTPUT"
if [[ "$OUTPUT" == "$RAND_CONTENT" ]]; then
    ok "✓ Blob 内容与预期一致"
else
    fail "✗ Blob 内容不匹配!\n预期: $RAND_CONTENT\n实际: $OUTPUT"
fi

# 清理测试文件
print_step "清理测试文件"
rm -f "$TEST_FILE"
ok "✓ 测试文件已清理"

# 显示测试结果
bold "\n✅ Blob 文件操作测试全部通过!"