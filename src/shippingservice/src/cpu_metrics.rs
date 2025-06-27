// Copyright The OpenTelemetry Authors
// SPDX-License-Identifier: Apache-2.0

use opentelemetry::{global, KeyValue};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt, PidExt};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;
use log::*;
use std::fs;

// CPU stats from cgroups (same as docker stats) for accurate container CPU
#[derive(Debug, Clone)]
struct CgroupCpuStats {
    usage_total: u64,
    system_usage: u64,
    online_cpus: u32,
}

impl CgroupCpuStats {
    fn from_cgroup() -> Result<Self, Box<dyn std::error::Error>> {
        // Try cgroups v2 first, then v1
        let (usage_total, system_usage, online_cpus) = 
            Self::try_cgroups_v2().or_else(|_| Self::try_cgroups_v1())?;
        
        Ok(CgroupCpuStats {
            usage_total,
            system_usage,
            online_cpus,
        })
    }
    
    fn try_cgroups_v2() -> Result<(u64, u64, u32), Box<dyn std::error::Error>> {
        // cgroups v2 path
        let usage_str = fs::read_to_string("/sys/fs/cgroup/cpu.stat")?;
        let mut usage_total = 0u64;
        
        for line in usage_str.lines() {
            if line.starts_with("usage_usec ") {
                usage_total = line.split_whitespace().nth(1)
                    .ok_or("Invalid cpu.stat format")?
                    .parse::<u64>()? * 1000; // Convert microseconds to nanoseconds
                break;
            }
        }
        
        if usage_total == 0 {
            return Err("Could not find usage_usec in cpu.stat".into());
        }
        
        // System CPU usage from /proc/stat
        let stat_content = fs::read_to_string("/proc/stat")?;
        let first_line = stat_content.lines().next().ok_or("Empty /proc/stat")?;
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        
        if parts.len() < 8 {
            return Err("Invalid /proc/stat format".into());
        }
        
        let system_usage: u64 = parts[1..8].iter()
            .map(|s| s.parse::<u64>().unwrap_or(0))
            .sum::<u64>() * 10_000_000; // Convert to nanoseconds (assuming 100Hz)
        
        let online_cpus = Self::get_online_cpus()?;
        
        Ok((usage_total, system_usage, online_cpus))
    }
    
    fn try_cgroups_v1() -> Result<(u64, u64, u32), Box<dyn std::error::Error>> {
        // cgroups v1 paths
        let usage_total = fs::read_to_string("/sys/fs/cgroup/cpu,cpuacct/cpuacct.usage")?
            .trim().parse::<u64>()?;
        
        // System CPU usage from /proc/stat  
        let stat_content = fs::read_to_string("/proc/stat")?;
        let first_line = stat_content.lines().next().ok_or("Empty /proc/stat")?;
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        
        if parts.len() < 8 {
            return Err("Invalid /proc/stat format".into());
        }
        
        let system_usage: u64 = parts[1..8].iter()
            .map(|s| s.parse::<u64>().unwrap_or(0))
            .sum::<u64>() * 10_000_000; // Convert to nanoseconds
        
        let online_cpus = Self::get_online_cpus()?;
        
        Ok((usage_total, system_usage, online_cpus))
    }
    
    fn get_online_cpus() -> Result<u32, Box<dyn std::error::Error>> {
        // Try to get CPU count from cgroup limits first
        if let Ok(quota_str) = fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_quota_us") {
            if let Ok(period_str) = fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_period_us") {
                let quota: i64 = quota_str.trim().parse().unwrap_or(-1);
                let period: u64 = period_str.trim().parse().unwrap_or(100000);
                
                if quota > 0 {
                    return Ok((quota as u64 / period).max(1) as u32);
                }
            }
        }
        
        // Fallback to system CPU count
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
        let cpu_count = cpuinfo.lines()
            .filter(|line| line.starts_with("processor"))
            .count() as u32;
        
        Ok(cpu_count.max(1))
    }
}

// Shared state for CPU metrics
struct CpuMetricsState {
    system: System,
    current_pid: sysinfo::Pid,
    container_cpu_usage: f64,
    process_cpu_usage: f64,
    process_memory_usage: u64,
    last_cgroup_stats: Option<CgroupCpuStats>,
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
            last_cgroup_stats: None,
            last_measurement_time: Instant::now(),
        })
    }

    fn refresh(&mut self) {
        self.system.refresh_all();
        
        // Calculate container CPU usage using cgroups (same as docker stats)
        match CgroupCpuStats::from_cgroup() {
            Ok(current_stats) => {
                if let Some(ref last_stats) = self.last_cgroup_stats {
                    let container_usage_diff = current_stats.usage_total - last_stats.usage_total;
                    let system_usage_diff = current_stats.system_usage - last_stats.system_usage;
                    
                    if system_usage_diff > 0 {
                        // Calculate CPU usage percentage (same method as docker stats)
                        let cpu_percent = (container_usage_diff as f64 / system_usage_diff as f64) 
                            * current_stats.online_cpus as f64 * 100.0;
                        
                        self.container_cpu_usage = cpu_percent.min(100.0).max(0.0);
                        
                        info!("Cgroup CPU calculation - Container: {} ns, System: {} ns, CPUs: {}, Usage: {:.2}%", 
                               container_usage_diff, system_usage_diff, current_stats.online_cpus, self.container_cpu_usage);
                    }
                } else {
                    // First measurement
                    self.container_cpu_usage = 0.0;
                    info!("First cgroup CPU measurement taken");
                }
                self.last_cgroup_stats = Some(current_stats);
            }
            Err(e) => {
                warn!("Failed to read cgroup CPU stats: {}", e);
                // Fallback to /proc/stat method
                self.container_cpu_usage = self.system.global_cpu_info().cpu_usage() as f64;
                debug!("Using fallback CPU calculation: {:.2}%", self.container_cpu_usage);
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
        .with_description("Container CPU usage percentage using cgroups (same as docker stats)")
        .with_callback(move |observer| {
            if let Ok(state) = state_clone1.lock() {
                observer.observe(
                    state.container_cpu_usage,
                    &[
                        KeyValue::new("service.name", "shippingservice"),
                        KeyValue::new("metric.type", "container_cpu"),
                        KeyValue::new("scope", "container"),
                        KeyValue::new("calculation_method", "cgroups"),
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

    info!("CPU metrics observable gauges registered successfully (using cgroups like docker stats)");

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
            info!("Updated metrics - Container CPU: {:.2}% (cgroup-based, matches docker stats), Process CPU: {:.2}%, Memory: {} bytes", 
                   state.container_cpu_usage, state.process_cpu_usage, state.process_memory_usage);
        }
    }
}
