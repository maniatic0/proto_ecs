#[cfg(test)]
mod tests {
    use crate::utils::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("A"), "a".to_string());
        assert_eq!(to_camel_case(""), "".to_string());
        assert_eq!(to_camel_case("DataGroup"), "data_group".to_string());
        assert_eq!(to_camel_case("aaa"), "aaa".to_string());
        assert_eq!(to_camel_case("A123Sa456"), "a_123_sa_456".to_string());
        assert_eq!(to_camel_case("AA"), "a_a".to_string());
    }
}
