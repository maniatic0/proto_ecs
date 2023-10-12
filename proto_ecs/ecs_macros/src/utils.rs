/// Utility functions

pub fn to_camel_case(s: &str) -> String {
    if s.len() == 0 {
        return "".to_string();
    }
    // Shortcut lambda to lowercase a char
    let to_lowercase = |c: u8| (c as char).to_ascii_lowercase();

    let s_bytes = s.as_bytes();
    let mut result = String::with_capacity(s_bytes.len());

    // First letter is always lowercase
    result.push(to_lowercase(s_bytes[0]));

    for &ch in &s_bytes[1..] {
        let ch = ch as char;
        if ch.is_uppercase() {
            result.push('_');
        }

        result.push(ch.to_ascii_lowercase());
    }

    result
}
