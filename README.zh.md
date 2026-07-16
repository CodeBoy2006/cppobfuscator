# cppobfuscator

[English](README.md) | 中文

`cppobfuscator` 是一个正确性优先的 C++ 单翻译单元混淆器，主要面向单文件竞赛代码。它使用 Tree-sitter C++ 语法节点、词法作用域和字节区间编辑，不再进行全局文本替换。

> 仅在竞赛和平台规则允许时使用。生成代码的编译、测试与提交责任由使用者承担。

## 设计

转换流程有意保持克制：

1. 使用 `tree-sitter-cpp` 解析原始源码。
2. 建立词法作用域、声明点、重载组和标识符引用。
3. 生成互不重叠的字节区间重命名编辑。
4. 重新解析，并要求非注释 AST 结构与原始源码一致。
5. 按配置移除注释和/或压缩布局。
6. 再次解析并校验 AST 结构。

`0.2` 版本主动删除了旧的文本级宏展开、函数内联、整数常量提升和未使用代码删除。这些转换可能改变求值顺序、重载解析、初始化副作用或合法模板代码。

## 重命名策略

只有词法模型能够稳定解析的符号才会被重命名：

- 局部变量、结构化绑定、lambda 初始化捕获和参数。
- 普通自由函数及其重载组。
- 引用关系明确的文件级和命名空间变量。

以下名称会保守保留：

- `main`、用户指定名称、无法解析的名称和保留标识符。
- 类型、命名空间、字段、成员函数、运算符、标签和限定名称。
- `extern` 与语言链接实体。
- 通过 `friend` 声明的命名空间函数。
- 宏名称，以及宏替换文本中不是宏参数的标识符。

生成名称仅使用小写字母和数字，不会与源码中已有标识符冲突；相同种子会产生确定性结果。

## 源码处理边界

- 不执行宏展开，也不改写宏替换文本。
- 对替换体以 `for`、`if`、`while` 或 `switch` 开头的常见函数式语句宏做保守兼容。
- 保留多行宏的反斜杠续行。
- 原样保留原始字符串、UTF-8 文本、数字分隔符和用户定义字面量后缀。
- 拒绝使用 `##` 或 `%:%:` 的 token-pasting 宏，因为它能生成 AST 中不存在的标识符。

Tree-sitter 是语法解析器，不是 C++ 编译器或预处理器。生成文件必须使用与提交环境相同的编译器参数重新编译和测试。

## 命令行

```text
cppobfuscator [OPTIONS]

-i, --input <PATH>       从文件读取 C++ 源码（默认：stdin）
-o, --output <PATH>      写入文件（默认：stdout）
    --seed <U64>         重命名种子，支持十进制或 0x 前缀（默认：0xC0FFEE）
    --preserve <NAME>    保留标识符，可重复指定
    --keep-comments      保留注释
    --keep-layout        保留原始空白和行布局
-h, --help               显示帮助
-V, --version            显示版本
```

文件输入输出：

```bash
cppobfuscator -i solution.cpp -o solution.obfuscated.cpp
```

使用 stdin/stdout：

```bash
cppobfuscator --seed 42 < solution.cpp > solution.obfuscated.cpp
```

保留外部要求的函数名：

```bash
cppobfuscator --preserve solve -i solution.cpp -o solution.obfuscated.cpp
```

## Rust API

```rust
use cppobfuscator::{Options, obfuscate};

let output = obfuscate(source, &Options::default())?;
```

## 开发验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
```
