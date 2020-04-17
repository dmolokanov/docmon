use serde::Serialize;

mod collect;
mod emit;

pub use collect::Collector;

#[derive(Debug, Clone, Serialize)]
pub struct Stats;
