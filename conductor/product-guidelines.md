# Product Guidelines - AbyssWatcher

## Design Principles
- **Technical and Precise:** The user interface and documentation should prioritize data accuracy and detailed metrics. The aesthetic should be functional and "no-nonsense," catering to hardcore players who value information over fluff.
- **Data-Dense and Compact:** Layouts must be optimized to display the maximum amount of relevant information in the smallest possible footprint. This is critical for multiboxers who need to monitor several characters without obscuring the main game client.
- **Immediate Cognitive Clarity:** Use visual indicators like color-coding (e.g., green for dealt damage, red for received), trend arrows, and progress bars to allow users to grasp combat status at a glance during high-intensity gameplay.

## Communication & Feedback
- **Visual-First Status:** Primary combat metrics (DPS, total damage, etc.) should be represented through visual elements that support rapid interpretation.
- **Context-Aware Audio Alerts (Future):** The system should eventually include programmable sound cues for specific operational failures, such as a character becoming inactive (not shooting) while the rest of the fleet is engaged in combat.
- **Detailed Numerical Access:** While the overlay is compact, raw numerical data should be accessible (e.g., via tooltips or a secondary view) for precise post-combat review.

## Data Integrity & Accuracy
- **Strict Log Validation:** AbyssWatcher follows a "truth-in-data" policy. Only log entries that can be parsed with 100% certainty should be processed and displayed. Any ambiguous or corrupted log lines should be flagged or logged internally rather than guessed at, ensuring the user can always trust the displayed metrics.
- **No-Inference Policy:** Avoid approximating missing data. If a log file is incomplete or delayed, the UI should clearly reflect the state of the data as it exists on disk.
