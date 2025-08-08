#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::task;
    use insta::assert_json_snapshot;

    #[tokio::test]
    async fn test_basic_flow() {
        let result = perform_operation("valid_input").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "expected_output");
    }

    #[tokio::test]
    async fn test_empty_input() {
        let result = perform_operation("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_malformed_input() {
        let result = perform_operation("{bad json}").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_snapshot_output() {
        let result = perform_operation("structured_input").await.unwrap();
        assert_json_snapshot!(result);
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        let handles: Vec<_> = (0..10)
            .map(|_| task::spawn(perform_operation("valid_input")))
            .collect();

        for handle in handles {
            let res = handle.await.unwrap();
            assert!(res.is_ok());
        }
    }
}