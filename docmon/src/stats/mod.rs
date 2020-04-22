use std::convert::TryFrom;

use anyhow::anyhow;
use serde::Serialize;

mod collect;
mod emit;

pub use collect::Collector;

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    timestamp: chrono::DateTime<chrono::Utc>,
    id: String,
    name: String,
    cpu_percentage: Option<f64>,
    memory: Option<u64>,
    memory_percentage: Option<f64>,
    memory_limit: Option<u64>,
    network_rx: Option<u64>,
    network_tx: Option<u64>,
    block_read: Option<u64>,
    block_write: Option<u64>,
    pid: Option<u64>,
}

impl TryFrom<bollard::container::Stats> for Stats {
    type Error = anyhow::Error;

    fn try_from(stats: bollard::container::Stats) -> Result<Self, Self::Error> {
        if stats.read < stats.preread {
            return Err(anyhow!("current measurement unavailable"));
        }

        let cpu_delta = (stats.cpu_stats.cpu_usage.total_usage
            - stats.precpu_stats.cpu_usage.total_usage) as f64;

        let online_cpus = match stats.cpu_stats.online_cpus {
            Some(cpus) => cpus as f64,
            None => stats
                .cpu_stats
                .cpu_usage
                .percpu_usage
                .as_ref()
                .map_or(0, |usage| usage.len()) as f64,
        };

        let cpu_percentage = stats
            .cpu_stats
            .system_cpu_usage
            .and_then(|usage| {
                stats
                    .precpu_stats
                    .system_cpu_usage
                    .map(|previous| (usage - previous) as f64)
            })
            .map(|system_delta| (cpu_delta / system_delta) * online_cpus * 100.0);

        // DO NOT REMOVE this block calculates memory usage as docker cli master
        // let memory = stats
        //     .memory_stats
        //     .stats
        //     .map(|stats| stats.total_inactive_file)
        //     .or_else(|| stats.memory_stats.stats.map(|stats| stats.inactive_file))
        //     .and_then(|inactive| {
        //         stats
        //             .memory_stats
        //             .usage
        //             .filter(|usage| &inactive < usage)
        //             .map(|usage| usage - inactive)
        //     })
        //     .or(stats.memory_stats.usage);
        let memory = stats
            .memory_stats
            .usage
            .and_then(|usage| stats.memory_stats.stats.map(|stats| usage - stats.cache));

        let memory_limit = stats.memory_stats.limit;

        let memory_percentage = memory
            .and_then(|memory| memory_limit.map(|limit| (memory as f64) / (limit as f64) * 100.0));

        let (network_rx, network_tx) = stats.networks.map_or((None, None), |networks| {
            let (rx, tx) = networks.values().fold((0, 0), |(rx, tx), stats| {
                (rx + stats.rx_bytes, tx + stats.tx_bytes)
            });
            (Some(rx), Some(tx))
        });

        let (block_read, block_write) =
            stats
                .blkio_stats
                .io_service_bytes_recursive
                .map_or((None, None), |entries| {
                    let (read, write) = entries.iter().fold((0, 0), |(read, write), stats| {
                        match stats.op.as_str() {
                            "r" | "R" => (read + stats.value, write),
                            "w" | "W" => (read, write + stats.value),
                            _ => (read, write),
                        }
                    });
                    (Some(read), Some(write))
                });

        Ok(Self {
            timestamp: stats.read,
            id: stats.id,
            name: stats.name,
            cpu_percentage,
            memory,
            memory_percentage,
            memory_limit,
            network_rx,
            network_tx,
            block_read,
            block_write,
            pid: stats.pids_stats.current,
        })
    }
}
