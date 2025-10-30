// Property-based tests for parse_dataset_path using proptest

use proptest::prelude::*;

proptest! {
    // Owner and dataset should not contain slashes and cannot be empty
    #[test]
    fn prop_parse_dataset_path_never_accepts_empty_or_slash_only(
        owner in proptest::string::string_regex(r"[A-Za-z0-9_. -]{0,20}").unwrap(),
        dataset in proptest::string::string_regex(r"[A-Za-z0-9_. -]{0,20}").unwrap()
    ) {
        let input = format!("{}/{}", owner, dataset);
        let res = gaggle::parse_dataset_path(&input);
        if owner.is_empty() || dataset.is_empty() || owner.contains('/') || dataset.contains('/') {
            prop_assert!(res.is_err());
        } else {
            let ok = res.unwrap();
            prop_assert_eq!(ok.0, owner);
            prop_assert_eq!(ok.1, dataset);
        }
    }
}
