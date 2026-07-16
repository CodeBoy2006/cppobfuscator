# cppobfuscator

[English](README.md) | 中文

`cppobfuscator` 是一个面向单文件竞赛程序的低开销 C++ 源码混淆器。它结合
Tree-sitter 作用域分析、AST 字节区间重写和编译期词法变换，不展开头文件或宏，
也不进行全局文本替换。

> 仅在竞赛和平台规则允许时使用。生成文件必须使用提交环境的编译器参数重新编译
> 和测试。

## 混淆档位

`0.3` 提供三个确定性档位：

| 档位 | 变换 |
| --- | --- |
| `symbols` | 重命名可解析的变量、自由函数、模板参数和标签；按配置压缩布局、删除源码注释。 |
| `balanced` | 默认档。增加函数间局部短名复用、安全整数进制变换、ASCII 字符串/字符八进制转义和 C++ 替代运算符。 |
| `maximum` | 增加视觉相近的 `i/l/o/0/1` 名称，并在非预处理 token 之间插入经过复验的分隔注释。 |

高级档位只改变编译期源码表示，不加入运行时解码器、不透明分支、控制流调度器、
堆分配或额外初始化。代表性计算内核在变换前后生成了字节完全一致的
`g++ -O2` 汇编。

## 处理流程

1. 使用 `tree-sitter-cpp` 解析 UTF-8 单翻译单元。
2. 收集源码标识符、宏依赖、链接约束和 C++ 名字查找风险。
3. 建立词法作用域、声明点、重载组、模板参数、标签和标识符引用。
4. 应用互不重叠的符号编辑并重新解析。
5. 改写安全字面量和表达式运算符拼写并重新解析。
6. 删除源码注释、压缩布局，同时保留全部物理换行和宏续行。
7. 再次解析，并要求规范化后的非注释 AST 结构一致。

## 符号混淆

分析器会重命名：

- 引用关系明确的局部、文件级和命名空间变量。
- 函数参数、结构化绑定和 lambda 初始化捕获。
- 普通自由函数及同作用域重载组。
- 类型和非类型模板参数，包括依赖限定名中的引用。
- 标准 `goto` 标签及其引用。

无法证明安全时会保留名称：

- `main`、显式 `--preserve` 名称、保留标识符和无法解析的名称。
- 类型、命名空间、字段、成员函数、运算符、枚举项和限定成员名。
- 外部链接、语言链接和通过 `friend` 声明的函数。
- 宏名称，以及宏替换文本使用的非参数标识符。
- 不同作用域中的同名函数，以及存在 `using namespace` 时被调用的源码函数。
- 带基类的类中需要继承成员查找才能判断的裸外层名称。

`balanced` 和 `maximum` 会在互相独立的函数中复用同一组局部短名，同时保留所有
翻译单元级生成名称。`maximum` 使用视觉相近的小写名称；生成名称不会与源码中
已有标识符冲突。

## 词法混淆

- 有符号 32 位范围内的整数可以改写为二进制、八进制或十六进制，并保留后缀。
  浮点数、大整数和用户定义字面量保持不变。
- 普通字符串中的可打印 ASCII 内容使用固定宽度八进制转义。已有转义、原始字符串
  和非 ASCII 文本保持不变。
- 可打印 ASCII 字符字面量使用八进制转义。
- 表达式运算符可以使用 `and`、`or`、`not`、`bitand`、`bitor`、`xor`、
  `compl`、`and_eq`、`or_eq`、`xor_eq` 和 `not_eq`。指针/引用语法和一元
  取地址仍使用符号形式。
- `maximum` 会把符合条件的横向分隔替换为 `/**/` 或 `/*_*/`，并避开预处理行、
  替代运算符边界和除法边界。

布局压缩保留物理换行总数，因此 `__LINE__` 仍能观察到原始行号。
`--keep-layout` 会禁用布局压缩和分隔注释插入。

由于可能生成二进制整数字面量，`balanced` 和 `maximum` 产物要求 C++14 或更高标准。

## 宏边界

- 不重写或展开宏替换文本。
- 对替换体以 `for`、`if`、`while` 或 `switch` 开头的函数式语句宏，允许
  Tree-sitter 只产生对应的缺失分号节点。
- 保留反斜杠续行。
- 拒绝使用 `##` 或 `%:%:` 的 token-pasting 宏。
- 拒绝使用 `#` 或 `%:` 的函数式字符串化宏，因为参数重命名会改变生成字符串。

工具无法看到包含头文件中的定义。头文件提供的字符串化、token-pasting、罕见的
小写宏、编译器扩展以及完整 ADL/类型查找仍然超出当前模型。

## 命令行

```text
cppobfuscator [OPTIONS]

-i, --input <PATH>       从文件读取 C++ 源码（默认：stdin）
-o, --output <PATH>      写入文件（默认：stdout）
    --seed <U64>         重命名种子，支持十进制或 0x 前缀（默认：0xC0FFEE）
    --preserve <NAME>    保留标识符，可重复指定
    --profile <NAME>     symbols、balanced 或 maximum（默认：balanced）
    --keep-comments      保留源码注释
    --keep-layout        保留原始空白和行布局
-h, --help               显示帮助
-V, --version            显示版本
```

```bash
cppobfuscator --profile maximum -i solution.cpp -o solution.obfuscated.cpp
```

## Rust API

```rust
use cppobfuscator::{Options, Profile, obfuscate};

let options = Options {
    profile: Profile::Maximum,
    ..Options::default()
};
let output = obfuscate(source, &options)?;
```

## 开发验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build --release
```
