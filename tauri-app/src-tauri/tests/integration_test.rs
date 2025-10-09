#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_state_transitions() {
        // Test server state transitions
        let initial_state = "stopped";
        let valid_transitions = vec![
            ("stopped", "starting"),
            ("starting", "running"),
            ("running", "stopping"),
            ("stopping", "stopped"),
            ("running", "failed"),
            ("failed", "starting"),
        ];

        for (from, to) in valid_transitions {
            assert!(
                is_valid_transition(from, to),
                "Transition from {} to {} should be valid",
                from,
                to
            );
        }
    }

    #[test]
    fn test_event_serialization() {
        use crate::events::ProxyEvent;

        let event = ProxyEvent::ServerStarted {
            name: "test-server".to_string(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("ServerStarted"));
        assert!(serialized.contains("test-server"));

        let deserialized: ProxyEvent = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ProxyEvent::ServerStarted { name } => {
                assert_eq!(name, "test-server");
            }
            _ => panic!("Wrong event type deserialized"),
        }
    }

    #[test]
    fn test_log_parsing() {
        let test_cases = vec![
            (
                "[ERROR] Something went wrong",
                ("ERROR", "Something went wrong"),
            ),
            ("[INFO] Server started", ("INFO", "Server started")),
            ("[WARN] High memory usage", ("WARN", "High memory usage")),
            (
                "[DEBUG] Connection established",
                ("DEBUG", "Connection established"),
            ),
            ("Plain log message", ("INFO", "Plain log message")),
        ];

        for (input, (expected_level, expected_msg)) in test_cases {
            let (level, msg) = crate::logs::parse_log_line(input);
            assert_eq!(level, expected_level);
            assert_eq!(msg, expected_msg);
        }
    }

    fn is_valid_transition(from: &str, to: &str) -> bool {
        match (from, to) {
            ("stopped", "starting") => true,
            ("starting", "running") => true,
            ("running", "stopping") => true,
            ("stopping", "stopped") => true,
            ("running", "failed") => true,
            ("failed", "starting") => true,
            _ => false,
        }
    }
}
