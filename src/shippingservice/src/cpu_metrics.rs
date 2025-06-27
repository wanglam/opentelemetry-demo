// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use opentelemetry::{global, KeyValue};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt, PidExt};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;
use log::*;
use std::fs;

// CPU stats from /proc/stat for more accurate container CPU calculation
#[derive(Debug, Clone)]
struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuStats {
    fn from_proc_stat() -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string("/proc/stat")?;
        let first_line = content.lines().next().ok_or("Empty /proc/stat")?;
        
        if !first_line.starts_with("cpu ") {
            return Err("Invalid /proc/stat format".into());
        }
        
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 9 {
            return Err("Insufficient CPU stats in /proc/stat".into());
        }
        
        Ok(CpuStats {
            user: parts[1].parse()?,
            nice: parts[2].parse()?,
            system: parts[3].parse()?,
            idle: parts[4].parse()?,
            iowait: parts[5].parse()?,
            irq: parts[6].parse()?,
            softirq: parts[7].parse()?,
            steal: parts[8].parse()?,
        })
    }
    
    fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal
    }
    
    fn idle_total(&self) -> u64 {
        self.idle + self.iowait
    }
    
    fn active_total(&self) -> u64 {
        self.total() - self.idle_total()
    }
}

// Shared state for CPU metrics
struct CpuMetricsState {
    system: System,
    current_pid: sysinfo::Pid,
    container_cpu_usage: f64,
    process_cpu_usage: f64,
    process_memory_usage: u64,
    last_cpu_stats: Option<CpuStats>,
    last_measurement_time: Instant,
}

impl CpuMetricsState {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let current_pid = sysinfo::get_current_pid()
            .map_err(|e| format!("Failed to get current process ID: {}", e))?;
        
        Ok(CpuMetricsState {
            system: System::new_all(),
            current_pid,
            container_cpu_usage: 0.0,
            process_cpu_usage: 0.0,
            process_memory_usage: 0,
            last_cpu_stats: None,
            last_measurement_time: Instant::now(),
        })
    }

    fn refresh(&mut self) {
        self.system.refresh_all();
        
        // Calculate accurate container CPU usage using /proc/stat
        match CpuStats::from_proc_stat() {
            Ok(current_stats) => {
                if let Some(ref last_stats) = self.last_cpu_stats {
                    let total_diff = current_stats.total() - last_stats.total();
                    let active_diff = current_stats.active_total() - last_stats.active_total();
                    
                    if total_diff > 0 {
                        self.container_cpu_usage = (active_diff as f64 / total_diff as f64) * 100.0;
                        
                        // Debug information for troubleshooting
                        debug!("CPU calculation - Total diff: {}, Active diff: {}, Usage: {:.2}%", 
                               total_diff, active_diff, self.container_cpu_usage);
                    }
                } else {
                    // First measurement, can't calculate diff yet
                    self.container_cpu_usage = 0.0;
                    info!("First CPU measurement taken, next measurement will show usage");
                }
                self.last_cpu_stats = Some(current_stats);
            }
            Err(e) => {
                warn!("Failed to read /proc/stat for accurate CPU usage: {}", e);
                // Fallback to sysinfo (less accurate in containers)
                let fallback_usage = self.system.global_cpu_info().cpu_usage() as f64;
                self.container_cpu_usage = fallback_usage;
                debug!("Using fallback CPU calculation: {:.2}%", fallback_usage);
            }
        }
        
        // Update process-specific metrics (these are accurate)
        if let Some(process) = self.system.process(self.current_pid) {
            self.process_cpu_usage = process.cpu_usage() as f64;
            self.process_memory_usage = process.memory();
        }
        
        self.last_measurement_time = Instant::now();
    }
}

pub async fn start_cpu_metrics_collection() {
    let cpu_state = match CpuMetricsState::new() {
        Ok(state) => Arc::new(Mutex::new(state)),
        Err(e) => {
            error!("Failed to initialize CPU metrics: {}", e);
            return;
        }
    };

    let meter = global::meter("shippingservice");
    
    // Create observable gauges with callbacks
    let state_clone1 = Arc::clone(&cpu_state);
    let _container_cpu_gauge = meter
        .f64_observable_gauge("container_cpu_usage")
        .with_description("Accurate container CPU usage percentage calculated from /proc/stat")
        .with_callback(move |observer| {
            if let Ok(state) = state_clone1.lock() {
                observer.observe(
                    state.container_cpu_usage,
                    &[
                        KeyValue::new("service.name", "shippingservice"),
                        KeyValue::new("metric.type", "container_cpu"),
                        KeyValue::new("scope", "container"),
                        KeyValue::new("calculation_method", "proc_stat"),
                    ],
                );
            }
        })
        .init();

    let state_clone2 = Arc::clone(&cpu_state);
    let current_pid = cpu_state.lock().unwrap().current_pid;
    let _process_cpu_gauge = meter
        .f64_observable_gauge("process_cpu_usage")
        .with_description("Process CPU usage percentage")
        .with_callback(move |observer| {
            if let Ok(state) = state_clone2.lock() {
                observer.observe(
                    state.process_cpu_usage,
                    &[
                        KeyValue::new("service.name", "shippingservice"),
                        KeyValue::new("process.pid", current_pid.as_u32() as i64),
                        KeyValue::new("process.name", "shippingservice"),
                        KeyValue::new("metric.type", "process_cpu"),
                    ],
                );
            }
        })
        .init();

    let state_clone3 = Arc::clone(&cpu_state);
    let _process_memory_gauge = meter
        .u64_observable_gauge("process_memory_usage")
        .with_description("Process memory usage in bytes")
        .with_callback(move |observer| {
            if let Ok(state) = state_clone3.lock() {
                observer.observe(
                    state.process_memory_usage,
                    &[
                        KeyValue::new("service.name", "shippingservice"),
                        KeyValue::new("process.pid", current_pid.as_u32() as i64),
                        KeyValue::new("process.name", "shippingservice"),
                        KeyValue::new("metric.type", "process_memory"),
                    ],
                );
            }
        })
        .init();

    info!("CPU metrics observable gauges registered successfully (using /proc/stat for accurate container CPU)");

    // Background task to refresh the metrics data
    let mut interval = interval(Duration::from_secs(5));
    info!("Starting CPU metrics collection for shipping service (PID: {})", current_pid.as_u32());
    
    // Initial refresh to set baseline
    if let Ok(mut state) = cpu_state.lock() {
        state.refresh();
    }
    
    loop {
        interval.tick().await;
        
        if let Ok(mut state) = cpu_state.lock() {
            state.refresh();
            info!("Updated metrics - Container CPU: {:.2}% (accurate), Process CPU: {:.2}%, Memory: {} bytes", 
                   state.container_cpu_usage, state.process_cpu_usage, state.process_memory_usage);
        }
    }
}
