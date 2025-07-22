//! Simple test to verify the test infrastructure works

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_math() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_string_operations() {
        let hello = "Hello";
        let world = "World";
        let combined = format!("{} {}", hello, world);
        assert_eq!(combined, "Hello World");
    }
}