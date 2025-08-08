#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_round_trip(s in ".*") {
            let encoded = encode(&s);
            let decoded = decode(&encoded);
            prop_assert_eq!(decoded, s);
        }

        #[test]
        fn test_idempotent(s in ".*") {
            let once = normalize(&s);
            let twice = normalize(&once);
            prop_assert_eq!(once, twice);
        }

        #[test]
        fn test_combined_property(s in "[a-zA-Z0-9]{1,100}") {
            let norm = normalize(&s);
            let encoded = encode(&norm);
            let decoded = decode(&encoded);
            prop_assert_eq!(normalize(&decoded), norm);
        }

        #[test]
        fn test_non_panicking(s in proptest::collection::vec(".*", 1..100)) {
            for item in &s {
                let _ = perform_fallible(item);
            }
        }
    }
}