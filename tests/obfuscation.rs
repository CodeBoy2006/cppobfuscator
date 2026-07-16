use std::collections::BTreeSet;

use cppobfuscator::{ObfuscationError, Options, Profile, obfuscate};

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
        profile: Profile::Maximum,
        ..Options::default()
    };
    let second = Options {
        seed: 8,
        profile: Profile::Maximum,
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

    let output = obfuscate(
        source,
        &Options {
            profile: cppobfuscator::Profile::Symbols,
            ..Options::default()
        },
    )
    .unwrap();

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

#[test]
fn renames_template_parameters_and_labels() {
    let source = r#"
#include <vector>
template <class TypeValue, int SizeValue, template <class> class ContainerValue>
int transformValue(ContainerValue<TypeValue> inputValue) {
    int localValue = SizeValue;
    if (localValue != 0 && !inputValue.empty()) goto doneLabel;
doneLabel:
    return localValue;
}
int main() { return transformValue<int, 1, std::vector>({}); }
"#;
    let options = Options {
        profile: Profile::Symbols,
        compact: false,
        strip_comments: false,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    for original in [
        "TypeValue",
        "SizeValue",
        "ContainerValue",
        "inputValue",
        "localValue",
        "doneLabel",
    ] {
        assert!(!output.contains(original), "{original} was not renamed");
    }
}

#[test]
fn reuses_local_short_names_between_functions() {
    let source = r#"
int firstFunction(int firstParameter) { return firstParameter; }
int secondFunction(int secondParameter) { return secondParameter; }
int main() { return firstFunction(1) + secondFunction(2); }
"#;
    let output = obfuscate(source, &Options::default()).unwrap();
    let parameters: Vec<&str> = output
        .lines()
        .filter(|line| line.starts_with("int ") && !line.starts_with("int main"))
        .filter_map(|line| line.split("(int ").nth(1))
        .filter_map(|tail| tail.split(')').next())
        .collect();

    assert_eq!(parameters.len(), 2);
    assert_eq!(parameters[0], parameters[1]);
}

#[test]
fn balanced_profile_obfuscates_literals_and_operators() {
    let source = r#"
#include <string>
int main() {
    std::string messageValue = "hello";
    char markerValue = 'A';
    int numberValue = 42;
    return numberValue != markerValue && !messageValue.empty();
}
"#;
    let output = obfuscate(source, &Options::default()).unwrap();

    assert!(!output.contains("\"hello\""));
    assert!(output.contains("\\150"));
    assert!(output.contains("'\\101'"));
    assert!(!output.contains("42"));
    assert!(output.contains("not_eq") || output.contains(" and ") || output.contains("not "));
}

#[test]
fn maximum_profile_inserts_valid_separator_comments() {
    let source = "int helperValue(int inputValue) { return inputValue + 1; }\n";
    let options = Options {
        profile: Profile::Maximum,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(output.contains("/**/"));
    assert!(!output.contains("helperValue"));
    assert!(!output.contains("inputValue"));
}

#[test]
fn compact_layout_preserves_physical_line_count() {
    let source = "\n// heading\n\nint main() {\n    return __LINE__;\n}\n";
    let output = obfuscate(source, &Options::default()).unwrap();

    assert_eq!(
        source.bytes().filter(|byte| *byte == b'\n').count(),
        output.bytes().filter(|byte| *byte == b'\n').count()
    );
}

#[test]
fn rejects_stringifying_macros() {
    let source = "#define STRINGIFY(x) #x\nint main(){return STRINGIFY(value)[0];}\n";
    let error = obfuscate(source, &Options::default()).unwrap_err();

    assert!(matches!(error, ObfuscationError::Unsupported(_)));
    assert!(error.to_string().contains("stringifying"));
}

#[test]
fn preserves_names_that_require_cpp_lookup() {
    let source = r#"
int inheritedName = 100;
struct Base { int inheritedName = 7; };
struct Derived : Base {
    int readValue() const { return inheritedName; }
};
namespace imported { long chooseValue(long) { return 7; } }
int chooseValue(int) { return 100; }
using namespace imported;
int main() {
    Derived itemValue;
    return itemValue.readValue() + chooseValue(1L);
}
"#;
    let options = Options {
        profile: Profile::Symbols,
        compact: false,
        strip_comments: false,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert_eq!(output.matches("inheritedName").count(), 3);
    assert_eq!(output.matches("chooseValue").count(), 3);
    assert!(!output.contains("itemValue"));
}

#[test]
fn preserves_cross_scope_adl_overload_sets() {
    let source = r#"
namespace domain {
struct Value {};
int chooseValue(Value) { return 7; }
}
int chooseValue(int) { return 100; }
int main() { return chooseValue(domain::Value{}); }
"#;
    let options = Options {
        profile: Profile::Symbols,
        compact: false,
        strip_comments: false,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert_eq!(output.matches("chooseValue").count(), 3);
}

#[test]
fn maximum_profile_handles_alternative_assignment_operators() {
    let source = r#"
int main() {
    int value = 3;
    int* pointer = &value;
    value &= 7;
    value |= 8;
    value ^= 1;
    return (~value | *pointer) ^ 2;
}
"#;
    let options = Options {
        profile: Profile::Maximum,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(output.contains("and_eq"));
    assert!(output.contains("or_eq"));
    assert!(output.contains("xor_eq"));
    assert!(output.contains("compl"));
    assert!(output.contains("bitor"));
    assert!(output.contains('&'));
}

#[test]
fn maximum_profile_keeps_division_comment_boundaries_safe() {
    let source = "int main(){int value=12;int divisor=3;return value / divisor;}\n";
    let options = Options {
        profile: Profile::Maximum,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(output.contains("/ "));
    assert!(!output.contains("//**/"));
    assert!(!output.contains("//*_*/"));
}

#[test]
fn maximum_profile_renames_single_argument_direct_initializers() {
    let source = r#"
#include <vector>
int main() {
    int itemCount = 3;
    std::vector<int> values(itemCount);
    return static_cast<int>(values.size());
}
"#;
    let options = Options {
        profile: Profile::Maximum,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(!output.contains("itemCount"));
    assert!(!output.contains("values"));
}

#[test]
fn maximum_profile_encodes_integral_constants_as_expressions() {
    let source = r#"
template <int Size> struct Buffer { int values[Size]; };
enum Limits : int { Limit = 7 };
int main() {
    static_assert(Limit == 7);
    Buffer<3> buffer{};
    switch (Limit) {
        case 7: return buffer.values[0] + 1000000007;
        default: return 1;
    }
}
"#;
    let options = Options {
        profile: Profile::Maximum,
        compact: false,
        strip_comments: false,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(output.matches("static_cast<").count() >= 5);
    assert!(output.contains("0xffffffffULL"));
}

#[test]
fn maximum_profile_preserves_zero_null_pointer_literals() {
    let source = "int main(){int* pointerValue=0;return pointerValue==0;}\n";
    let options = Options {
        profile: Profile::Maximum,
        compact: false,
        strip_comments: false,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(!output.contains("static_cast<"));
    assert!(output.contains("0b0") || output.contains("00") || output.contains("0x0"));
}

#[test]
fn maximum_profile_preserves_integer_suffix_types() {
    let source = r#"
int main() {
    auto unsignedValue = 7U;
    auto longValue = 8L;
    auto longLongValue = 9LL;
    auto unsignedLongLongValue = 10ULL;
    return unsignedValue + longValue + longLongValue + unsignedLongLongValue;
}
"#;
    let options = Options {
        profile: Profile::Maximum,
        compact: false,
        strip_comments: false,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    assert!(output.contains("static_cast<unsigned int>"));
    assert!(output.contains("static_cast<long>"));
    assert!(output.contains("static_cast<long long>"));
    assert!(output.contains("static_cast<unsigned long long>"));
}

#[test]
fn preserves_syntax_sensitive_string_literals() {
    let source = r#"
extern "C" int callbackValue(int);
[[deprecated("legacy message")]]
int helperValue(int inputValue) {
    asm volatile("nop");
    return inputValue;
}
static_assert(1 == 1, "compile-time message");
int main() { return helperValue(1); }
"#;
    let options = Options {
        profile: Profile::Maximum,
        compact: false,
        strip_comments: false,
        ..Options::default()
    };

    let output = obfuscate(source, &options).unwrap();

    for literal in [
        "\"C\"",
        "\"legacy message\"",
        "\"nop\"",
        "\"compile-time message\"",
    ] {
        assert!(
            output.contains(literal),
            "{literal} was unexpectedly encoded"
        );
    }
}
