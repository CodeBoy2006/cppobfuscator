# cppobfuscator

[English](README.md) | 中文

> ⚠️ 注意： 请遵守竞赛平台规则。若因滥用本工具导致违规，后果自负。

## 概述

`cppobfuscator` 是一个用 Rust 编写的单文件 C++ 混淆器，支持：

- 标识符重命名（可选简单 1-2 字符模式）。
- 空白压缩（minify）。
- 简单函数内联。
- 整数常量混淆（constlift）。
- 宏展开（内置展开器，可关闭）。
- 移除未使用的宏、函数、全局变量。
- 向导模式（交互输入）。

## 用法

```bash
cppobfuscator [options]
```

如果从源码运行：

```bash
cargo run -- [options]
```

### 默认向导行为

当不带任何参数时，会默认进入向导模式，并提示：

```
Enable simple-names? [y/N]:
```

随后粘贴 C++ 代码，输入结束标记行（默认：`END`）。

## 参数列表

- `-i, --input <path>`：输入 C++ 文件（默认 stdin）。
- `-o, --output <path>`：输出文件（默认 stdout）。
- `--seed <u64>`：重命名随机种子（默认：`0xC0FFEE`）。
- `--no-rename`：关闭标识符重命名。
- `--simple-names`：使用 1-2 字符重命名（会禁用 constlift）。
- `--no-minify`：保留原始空白与换行。
- `--no-inline`：关闭简单函数内联。
- `--no-constlift`：关闭整数常量混淆。
- `--keep-comments`：保留注释（默认会移除）。
- `--no-strip-unused-macros`：保留未使用的 `#define` 宏（默认移除）。
- `--no-strip-unused-functions`：保留未使用的函数定义（默认移除）。
- `--no-strip-unused-globals`：保留未使用的全局变量/对象（默认移除）。
- `--no-expand-macros`：关闭宏展开。
- `--preserve <name>`：保留指定标识符（可重复）。
- `--wizard`：向导模式（不能与 `--input` 同时使用）。
- `--wizard-end <marker>`：向导结束标记（默认：`END`）。
- `-h, --help`：显示帮助。

说明：
- `--simple-names` 不能与 `--no-rename` 同时使用。
- `--wizard-end` 会自动启用向导模式。

## 示例

混淆文件：

```bash
cppobfuscator -i test.cpp -o test_obf.cpp
```

启用简单命名：

```bash
cppobfuscator -i test.cpp -o test_obf.cpp --simple-names
```

保留注释与格式：

```bash
cppobfuscator -i test.cpp -o test_obf.cpp --keep-comments --no-minify
```

向导模式：

```bash
cppobfuscator
```

按提示输入并以 `END` 结束。
