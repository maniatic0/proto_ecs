#[cfg(test)]
mod tests {
    use crate::utils::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("A"), "a".to_string());
        assert_eq!(to_snake_case(""), "".to_string());
        assert_eq!(to_snake_case("DataGroup"), "data_group".to_string());
        assert_eq!(to_snake_case("aaa"), "aaa".to_string());
        assert_eq!(to_snake_case("A123Sa456"), "a_123_sa_456".to_string());
        assert_eq!(to_snake_case("AA"), "a_a".to_string());
    }
}
