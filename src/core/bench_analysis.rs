#[cfg(test)]
mod tests {
    use crate::core::analysis;
    use crate::core::model::{CombatEvent, EventType};
    use std::time::{Duration, Instant};

    fn generate_large_history(count: usize) -> Vec<CombatEvent> {
        let mut events = Vec::with_capacity(count);
        for i in 0..count {
            events.push(CombatEvent {
                timestamp: Duration::from_millis(i as u64 * 10), // 10ms gap
                source: "Player".to_string(),
                target: "Enemy".to_string(),
                weapon: "Gun".to_string(),
                amount: 10.0,
                incoming: false,
                character: "Char1".to_string(),
                event_type: EventType::Damage,
            });
        }
        events
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore)]
    fn benchmark_compute_dps_series_performance() {
        let event_count = 100_000;
        let events = generate_large_history(event_count);
        
        // We want a window near the end, which is the worst case for O(N) linear scan
        let last_ts = events.last().unwrap().timestamp;
        let window = Duration::from_secs(5);
        
        let start = Instant::now();
        
        for _ in 0..100 {
            let _ = analysis::compute_dps_series(&events, window, last_ts);
        }
        
        let duration = start.elapsed();
        println!("Time for 100 iterations with {} events: {:?}", event_count, duration);

        // In release mode, 100 iterations of 100k events should be very fast if O(log N)
        // 500ms is a safe threshold for CI/local runs.
        assert!(duration < Duration::from_millis(1000), "Performance is too slow! took {:?}", duration);
    }
}