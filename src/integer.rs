#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegerType {
    Int,
    UnsignedInt,
    Long,
    UnsignedLong,
    LongLong,
    UnsignedLongLong,
}

impl IntegerType {
    pub(crate) fn cpp_name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::UnsignedInt => "unsigned int",
            Self::Long => "long",
            Self::UnsignedLong => "unsigned long",
            Self::LongLong => "long long",
            Self::UnsignedLongLong => "unsigned long long",
        }
    }
}

pub(crate) struct IntegerLiteral<'source> {
    pub(crate) value: u128,
    pub(crate) suffix: &'source str,
}

impl<'source> IntegerLiteral<'source> {
    pub(crate) fn parse(text: &'source str) -> Option<Self> {
        if text.starts_with(['+', '-']) {
            return None;
        }

        let (radix, digits_start, valid_digit): (u32, usize, fn(char) -> bool) =
            if text.starts_with("0x") || text.starts_with("0X") {
                (16, 2, |character| character.is_ascii_hexdigit())
            } else if text.starts_with("0b") || text.starts_with("0B") {
                (2, 2, |character| matches!(character, '0' | '1'))
            } else if text.starts_with('0') && text.len() > 1 {
                (8, 1, |character| matches!(character, '0'..='7'))
            } else {
                (10, 0, |character| character.is_ascii_digit())
            };

        let mut digits_end = digits_start;
        for (offset, character) in text[digits_start..].char_indices() {
            if valid_digit(character) || character == '\'' {
                digits_end = digits_start + offset + character.len_utf8();
            } else {
                break;
            }
        }
        if digits_end == digits_start {
            return None;
        }

        let suffix = &text[digits_end..];
        if !valid_integer_suffix(suffix) {
            return None;
        }
        let digits: String = text[digits_start..digits_end]
            .chars()
            .filter(|character| *character != '\'')
            .collect();
        let value = u128::from_str_radix(&digits, radix).ok()?;
        Some(Self { value, suffix })
    }

    pub(crate) fn render(&self, radix: Radix) -> String {
        let (prefix, digits) = match radix {
            Radix::Binary => ("0b", format!("{:b}", self.value)),
            Radix::Octal => ("0", format!("{:o}", self.value)),
            Radix::Hexadecimal => ("0x", format!("{:x}", self.value)),
        };
        format!("{prefix}{digits}{}", self.suffix)
    }

    pub(crate) fn exact_arithmetic_type(&self) -> Option<IntegerType> {
        // The existing lexical pass already uses this bound to guarantee that
        // every supported radix selects the same first standard integer type.
        if self.value > i32::MAX as u128 {
            return None;
        }

        match self.suffix.to_ascii_lowercase().as_str() {
            "" => Some(IntegerType::Int),
            "u" => Some(IntegerType::UnsignedInt),
            "l" => Some(IntegerType::Long),
            "ul" | "lu" => Some(IntegerType::UnsignedLong),
            "ll" => Some(IntegerType::LongLong),
            "ull" | "llu" => Some(IntegerType::UnsignedLongLong),
            // C++23 size suffixes do not map to one portable built-in spelling.
            "z" | "uz" | "zu" => None,
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Radix {
    Binary,
    Octal,
    Hexadecimal,
}

fn valid_integer_suffix(suffix: &str) -> bool {
    matches!(
        suffix.to_ascii_lowercase().as_str(),
        "" | "u" | "l" | "ul" | "lu" | "ll" | "ull" | "llu" | "z" | "uz" | "zu"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer_suffixes_without_touching_floats() {
        let literal = IntegerLiteral::parse("1'024ULL").unwrap();
        assert_eq!(literal.value, 1024);
        assert_eq!(literal.suffix, "ULL");
        assert_eq!(
            literal.exact_arithmetic_type(),
            Some(IntegerType::UnsignedLongLong)
        );
        assert!(IntegerLiteral::parse("1.5").is_none());
        assert!(IntegerLiteral::parse("2'000_score").is_none());
    }

    #[test]
    fn excludes_values_without_a_portable_exact_type() {
        assert!(
            IntegerLiteral::parse("2147483648")
                .unwrap()
                .exact_arithmetic_type()
                .is_none()
        );
        assert!(
            IntegerLiteral::parse("7z")
                .unwrap()
                .exact_arithmetic_type()
                .is_none()
        );
    }
}
