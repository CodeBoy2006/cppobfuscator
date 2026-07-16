use std::collections::BTreeSet;

use cppobfuscator::{ObfuscationError, Options, obfuscate};

#[test]
fn transforms_contest_style_cpp_with_scoped_symbols() {
    let source = r#"
#include <vector>
#define LOOP(i,n) for(int i=0;i<(n);++i)
#define APPLY(x) helperValue(x)

long long helperValue(long long inputValue) {
    return inputValue * 2;
}

struct Node {
    int memberValue;
    int getValue() const { return memberValue; }
};

int main() {
    std::vector<int> valuesValue{1, 2, 3};
    long long totalValue = 0;
    LOOP(indexValue, valuesValue.size()) {
        auto [leftValue, rightValue] =
            std::pair<int, int>{valuesValue[indexValue], 1};
        totalValue += leftValue + rightValue;
    }
    auto lambdaValue = [baseValue = totalValue](int deltaValue) mutable {
        baseValue += deltaValue;
        return baseValue;
    };
    Node nodeValue{3};
    return APPLY(lambdaValue(nodeValue.getValue()));
}
"#;

    let output = obfuscate(source, &Options::default()).unwrap();

    assert!(output.contains("LOOP(indexValue"));
    assert!(output.contains("helperValue"));
    assert!(output.contains("memberValue"));
    assert!(output.contains("getValue"));
    for renamed in [
        "inputValue",
        "valuesValue",
        "totalValue",
        "leftValue",
        "rightValue",
        "lambdaValue",
        "baseValue",
        "deltaValue",
        "nodeValue",
    ] {
        assert!(!output.contains(renamed), "{renamed} was not renamed");
    }
}

#[test]
fn output_is_deterministic_for_a_seed() {
    let source =
        "int helperName(int inputName){return inputName;} int main(){return helperName(1);}";
    let first = Options {
        seed: 7,
        ..Options::default()
    };
    let second = Options {
        seed: 8,
        ..Options::default()
    };

    assert_eq!(obfuscate(source, &first), obfuscate(source, &first));
    assert_ne!(
        obfuscate(source, &first).unwrap(),
        obfuscate(source, &second).unwrap()
    );
}

#[test]
fn preserves_explicit_and_external_names() {
    let source = r#"
extern int externalValue;
extern "C" int callbackValue(int);
static int internalValue = 1;
int solveValue(int inputValue) {
    return callbackValue(externalValue) + internalValue + inputValue;
}
int main() {
    return solveValue(2);
}
"#;
    let options = Options {
        preserve: BTreeSet::from(["solveValue".to_string()]),
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(output.contains("externalValue"));
    assert!(output.contains("callbackValue"));
    assert!(output.contains("solveValue"));
    assert!(!output.contains("internalValue"));
    assert!(!output.contains("inputValue"));
}

#[test]
fn preserves_cpp_lexical_edge_cases() {
    let source = r#"
#include <string>
constexpr unsigned long long operator""_score(unsigned long long value) {
    return value;
}
#define TWICE(x) \
    ((x) + (x))
int main() {
    int unicodeValue = 1'000;
    auto resultValue = 2'000_score + TWICE(unicodeValue);
    const char* rawValue = R"tag( // not a comment /* still text */ )tag";
    std::string textValue = "你好";
    return static_cast<int>(resultValue + rawValue[0] + textValue.size());
}
"#;

    let output = obfuscate(source, &Options::default()).unwrap();

    assert!(output.contains("1'000"));
    assert!(output.contains("2'000_score"));
    assert!(output.contains("\\\n"));
    assert!(output.contains("// not a comment /* still text */"));
    assert!(output.contains("\"你好\""));
    for renamed in ["unicodeValue", "resultValue", "rawValue", "textValue"] {
        assert!(!output.contains(renamed));
    }
}

#[test]
fn layout_and_comment_options_are_independent() {
    let source =
        "// heading\nint helperName(int inputName) { /* body */\n    return inputName;\n}\n";
    let keep_all = Options {
        compact: false,
        strip_comments: false,
        ..Options::default()
    };
    let strip_only = Options {
        compact: false,
        strip_comments: true,
        ..Options::default()
    };

    let kept = obfuscate(source, &keep_all).unwrap();
    let stripped = obfuscate(source, &strip_only).unwrap();

    assert!(kept.contains("// heading"));
    assert!(kept.contains("/* body */"));
    assert!(kept.contains("\n    return "));
    assert!(!stripped.contains("heading"));
    assert!(!stripped.contains("body"));
    assert!(stripped.contains("\n    return "));
}

#[test]
fn rejects_identifier_synthesizing_macros() {
    let source = "#define JOIN(a,b) a ## b\nint main(){return 0;}\n";
    let error = obfuscate(source, &Options::default()).unwrap_err();

    assert!(matches!(error, ObfuscationError::Unsupported(_)));
}
