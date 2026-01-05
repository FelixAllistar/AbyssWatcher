#[cfg(test)]
mod sim_tests {
    use crate::core::tracker::TrackedGamelog;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn simulate_multi_character_logs() {
        let dir = tempdir().unwrap();
        let log_path_a = dir.path().join("Gamelog_A.txt");
        let log_path_b = dir.path().join("Gamelog_B.txt");

        let mut file_a = File::create(&log_path_a).unwrap();
        let mut file_b = File::create(&log_path_b).unwrap();

        writeln!(file_a, "Session Started: 2025.12.24 10:00:00").unwrap();
        writeln!(file_b, "Session Started: 2025.12.24 10:00:00").unwrap();

        let mut tracker_a = TrackedGamelog::new("PilotA", &log_path_a).unwrap();
        let mut tracker_b = TrackedGamelog::new("PilotB", &log_path_b).unwrap();

        // Write events
        writeln!(
            file_a,
            "[ 2025.12.24 10:00:05 ] (combat) 100 to TargetX - Small Lasers"
        )
        .unwrap();
        writeln!(
            file_b,
            "[ 2025.12.24 10:00:06 ] (combat) 50 to TargetY - Small Missiles"
        )
        .unwrap();

        let result_a = tracker_a.read_new_events().unwrap();
        let result_b = tracker_b.read_new_events().unwrap();

        assert_eq!(result_a.combat_events.len(), 1);
        assert_eq!(result_a.combat_events[0].character, "PilotA");
        assert_eq!(result_a.combat_events[0].amount, 100.0);

        assert_eq!(result_b.combat_events.len(), 1);
        assert_eq!(result_b.combat_events[0].character, "PilotB");
        assert_eq!(result_b.combat_events[0].amount, 50.0);
    }
}
