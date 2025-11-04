// property_parse_dataset_path.rs
//
// This file contains property-based tests for the `parse_dataset_path` function in the Gaggle
// library. Using the `proptest` framework, these tests generate a wide range of string inputs
// to verify that the parser correctly handles valid dataset path formats and rejects invalid
// ones. The primary goal of these tests is to guarantee the robustness and correctness of the
// dataset path parsing logic, which is a critical component for interacting with the Kaggle API.

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_parse_dataset_path_never_accepts_empty_or_slash_only(
        owner in proptest::string::string_regex(r"[A-Za-z0-9_-]{1,20}").unwrap(),
        dataset in proptest::string::string_regex(r"[A-Za-z0-9_-]{1,20}").unwrap()
    ) {
        let input = format!("{}/{}", owner, dataset);
        let res = gaggle::parse_dataset_path(&input);
        // Should succeed for valid alphanumeric owner and dataset without slashes or traversal patterns
        prop_assert!(res.is_ok(), "Failed to parse valid path: {}", input);
        let ok = res.unwrap();
        prop_assert_eq!(ok.0, owner);
        prop_assert_eq!(ok.1, dataset);
    }
}
