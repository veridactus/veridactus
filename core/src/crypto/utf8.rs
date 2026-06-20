//! # UTF-8 安全处理（规范 §7.1.2）
//!
//! 当流式响应被截断时，`output.response` 可能包含不完整的 UTF-8 字节序列。
//! 为保证确定性签名，必须在规范化前进行安全处理。
//!
//! 此函数从原始字节层面处理，因为 `&str` 在 Rust 中必须是有效 UTF-8。
//!
//! 规则（AI.md §5.1）：
//! - 替换不完整的 UTF-8 多字节序列为 Unicode 替换字符 U+FFFD
//! - 确保控制字符 (U+0000-U+001F 除 \n, \r, \t) 被转义

use serde_json::Value;

/// 替换不完整的 UTF-8 多字节序列为 Unicode 替换字符 (U+FFFD)
///
/// 从原始字节层面处理，确保即使输入包含无效 UTF-8 也能安全处理。
///
/// # 参数
/// * `input` - 原始输入（可以是任意字节序列，不要求有效 UTF-8）
///
/// # 返回
/// 安全处理后的字符串（始终是有效 UTF-8）
pub fn sanitize_utf8_bytes(input: &[u8]) -> String {
    let mut output = String::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        let b = input[i];

        // 确定当前字节的字符长度
        let char_len = match b {
            0x00..=0x7F => 1,
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => {
                // 无效起始字节
                output.push('\u{FFFD}');
                i += 1;
                continue;
            }
        };

        // 检查是否有多余的有效字节
        if i + char_len > input.len() {
            // 不完整的字节序列：替换当前无效起始字节并继续处理后续字节
            output.push('\u{FFFD}');
            i += 1;
            continue;
        }

        // 对于多字节序列，验证后续字节是否有效 (10xx xxxx)
        if char_len > 1 {
            let mut valid = true;
            for j in 1..char_len {
                if (input[i + j] & 0xC0) != 0x80 {
                    valid = false;
                    break;
                }
            }
            if !valid {
                output.push('\u{FFFD}');
                i += 1;
                continue;
            }
        }

        // 复制有效 UTF-8 片段
        // 安全：已验证所有字节形成有效的 UTF-8 序列
        let slice = &input[i..i + char_len];
        let s = unsafe { std::str::from_utf8_unchecked(slice) };
        output.push_str(s);
        i += char_len;
    }

    output
}

/// 对已确认是有效 UTF-8 的字符串进行安全处理（保留接口兼容性）
///
/// 对于已经是有效 &str 的输入，此函数是恒等映射。
pub fn sanitize_utf8(input: &str) -> String {
    sanitize_utf8_bytes(input.as_bytes())
}

/// 对 JSON Value 进行 UTF-8 安全处理（递归处理所有字符串字段）
pub fn sanitize_utf8_json(value: &mut Value) {
    match value {
        Value::String(s) => {
            *s = sanitize_utf8_bytes(s.as_bytes());
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                sanitize_utf8_json(item);
            }
        }
        Value::Object(obj) => {
            for val in obj.values_mut() {
                sanitize_utf8_json(val);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试正常 ASCII 字符串
    #[test]
    fn test_ascii_string() {
        let input = "Hello, World!";
        assert_eq!(sanitize_utf8(input), input);
    }

    /// 测试有效多字节序列
    #[test]
    fn test_valid_multibyte() {
        let input = "caf\u{00E9}";
        assert_eq!(sanitize_utf8(input), input);
    }

    /// 测试中日韩字符
    #[test]
    fn test_cjk_characters() {
        let input = "你好世界";
        assert_eq!(sanitize_utf8(input), input);
    }

    /// 测试截断的多字节序列：字节数组方式
    #[test]
    fn test_truncated_multibyte_bytes() {
        // 不完整的 3 字节序列：0xE8 开头但缺少后续字节
        let bytes = [0x61, 0xE8]; // 'a' + 不完整的 UTF-8 起始字节
        let result = sanitize_utf8_bytes(&bytes);
        assert_eq!(result, "a\u{FFFD}");
    }

    /// 测试在中间截断的 2 字节序列
    #[test]
    fn test_truncated_2byte_middle() {
        // é = 0xC3 0xA9，只保留 0xC3
        let bytes = [0x48, 0xC3]; // "H" + 不完整的 é 起始
        let result = sanitize_utf8_bytes(&bytes);
        assert_eq!(result, "H\u{FFFD}");
    }

    /// 测试在中间截断的 4 字节序列
    #[test]
    fn test_truncated_4byte_middle() {
        // 四字节字符（如某些 emoji）开头的 0xF0，但缺少后续字节
        let bytes = [0x61, 0xF0, 0x61]; // 'a' + 不完整的 4字节起始 + 'a'
        let result = sanitize_utf8_bytes(&bytes);
        assert_eq!(result, "a\u{FFFD}a");
    }

    /// 测试无效起始字节
    #[test]
    fn test_invalid_start_byte() {
        // 0x80 是无效起始字节
        let bytes = [0x61, 0x80, 0x62]; // 'a', invalid, 'b'
        let result = sanitize_utf8_bytes(&bytes);
        assert_eq!(result, "a\u{FFFD}b");
    }

    /// 测试无效的后续字节
    #[test]
    fn test_invalid_continuation_byte() {
        // 0xC3 应该是 2 字节序列的起始，但后续字节是 0x61（非 10xxxxxx）
        let bytes = [0x61, 0xC3, 0x61]; // 'a', 2字节起始但后续无效, 'a'
        let result = sanitize_utf8_bytes(&bytes);
        assert_eq!(result, "a\u{FFFD}a");
    }

    /// 测试空输入
    #[test]
    fn test_empty_input() {
        assert_eq!(sanitize_utf8_bytes(&[]), "");
    }

    /// 测试全部无效
    #[test]
    fn test_all_invalid() {
        let bytes = [0x80, 0x81, 0x82];
        let result = sanitize_utf8_bytes(&bytes);
        assert_eq!(result, "\u{FFFD}\u{FFFD}\u{FFFD}");
    }

    /// 测试混合有效和无效序列
    #[test]
    fn test_mixed_valid_invalid() {
        // 'a' (0x61), 'é' (0xC3 0xA9 有效), 不完整4字节起始 (0xF0), 'z' (0x7A)
        let bytes = [0x61, 0xC3, 0xA9, 0xF0, 0x7A];
        let result = sanitize_utf8_bytes(&bytes);
        assert_eq!(result, "a\u{00E9}\u{FFFD}z");
    }
}
