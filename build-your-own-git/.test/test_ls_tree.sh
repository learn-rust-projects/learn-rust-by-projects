#!/usr/bin/env bash
set -euo pipefail

bold() { echo -e "\033[1m$*\033[0m"; }
info() { echo -e "\033[36m[INFO]\033[0m $*"; }
ok() { echo -e "\033[32m[OK]\033[0m $*"; }
fail() { echo -e "\033[31m[FAIL]\033[0m $*" >&2; exit 1; }
print_step() { echo -e "\033[33m▶ $*\033[0m"; }

PROGRAM="$1"
TEST_DIR="test_ls_tree_$(date +%s)"
OUR_OUTPUT=$(mktemp)
GIT_OUTPUT=$(mktemp)

mkdir -p "$TEST_DIR" && cd "$TEST_DIR"

# ========= 用我们的程序 =========
print_step "使用 $PROGRAM 初始化并生成tree"
"$PROGRAM" init

echo "hello world" > file1
mkdir dir1 && echo "hello world" > dir1/file_in_dir_1 && echo "hello world" > dir1/file_in_dir_2
mkdir dir2 && echo "hello world" > dir2/file_in_dir_3

TREE_SHA=$($PROGRAM write-tree)
info "我们的程序生成的tree哈希: $TREE_SHA"
"$PROGRAM" ls-tree --name-only "$TREE_SHA" | sort > "$OUR_OUTPUT"

print_step "我们的程序 ls-tree 输出内容"
cat "$OUR_OUTPUT"

# ========= 用官方git =========
print_step "切换到官方git初始化并生成tree"
rm -rf .git
git init > /dev/null

# 将当前目录下所有文件添加到索引
git add .

# 注意：文件还在，不需要重新创建
TREE_SHA=$(git write-tree)
info "官方git生成的tree哈希: $TREE_SHA"
git ls-tree --name-only "$TREE_SHA" | sort > "$GIT_OUTPUT"

print_step "官方 git ls-tree 输出内容"
cat "$GIT_OUTPUT"

# ========= 比较结果 =========
print_step "比较我们实现和官方git的ls-tree输出"
if diff -u "$OUR_OUTPUT" "$GIT_OUTPUT"; then
    ok "✓ 我们的输出与官方git完全一致"
else
    fail "✗ 输出不一致，请检查实现"
fi

# ========= 清理 =========
cd ..
rm -rf "$TEST_DIR" "$OUR_OUTPUT" "$GIT_OUTPUT"
bold "\n✅ ls-tree 测试完成！"
