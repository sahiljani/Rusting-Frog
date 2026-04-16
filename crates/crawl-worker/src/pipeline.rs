// The crawl pipeline: frontier → fetch → parse → evaluate → write
// This module will orchestrate the crawl loop once wired up.
// For now it defines the pipeline stages as types.

pub struct PipelineConfig {
    pub max_concurrent_fetches: usize,
    pub urls_per_second: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_fetches: 5,
            urls_per_second: 5,
        }
    }
}
