#[cfg(test)]
mod tests {
    use crate::core::analysis;
    use crate::core::model::CombatEvent;
    use std::time::{Duration, Instant};

    fn generate_large_history(count: usize) -> Vec<CombatEvent> {
        let mut events = Vec::with_capacity(count);
        for i in 0..count {
            events.push(CombatEvent {
                timestamp: Duration::from_millis(i as u64 * 10), // 10ms gap
                source: "Player".to_string(),
                target: "Enemy".to_string(),
                weapon: "Gun".to_string(),
                damage: 10.0,
                incoming: false,
                character: "Char1".to_string(),
            });
        }
        events
    }

    #[test]
    fn benchmark_compute_dps_series_performance() {
        let event_count = 50_000;
        let events = generate_large_history(event_count);
        
        // We want a window near the end, which is the worst case for O(N) linear scan
        // window = 5 seconds
        // end = last event timestamp
        let last_ts = events.last().unwrap().timestamp;
        let window = Duration::from_secs(5);
        
        let start = Instant::now();
        
        // Run it multiple times to simulate a few seconds of 4Hz updates
        for _ in 0..100 {
            let _ = analysis::compute_dps_series(&events, window, last_ts);
        }
        
        let duration = start.elapsed();
        println!("Time for 100 iterations with {} events: {:?}", event_count, duration);

        // Fail if it takes too long (arbitrary threshold to establish baseline vs optimized)
        // With O(N), 50k events * 100 iterations is 5M ops.
        // On a fast machine this might be fast, but we want to see the difference.
        // Let's assert it is FAST (under 10ms for 100 calls would be great, but unlikely for O(N)).
        // Actually, for TDD "Red" phase, we can just assert it runs, or assert a strict limit we know it will fail.
        
        // Let's set a strict limit of 50ms for 100 iterations (0.5ms per call).
        // 50k items iteration might take > 0.5ms.
        assert!(duration < Duration::from_millis(50), "Performance is too slow! took {:?}", duration);
    }
}
