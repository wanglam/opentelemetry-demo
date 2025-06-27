# Cgroup-Based CPU Metrics - Matching Docker Stats

## The Real Issue

You're absolutely right that `docker stats` looks good! The problem was that our implementation was using a different calculation method than `docker stats`. Now I've updated it to use **the exact same method** that `docker stats` uses.

## What Docker Stats Actually Uses

`docker stats` doesn't use `/proc/stat` - it uses **cgroup statistics**:

### **Docker Stats Method:**
```bash
# Docker reads from cgroup files:
/sys/fs/cgroup/cpu,cpuacct/cpuacct.usage     # Container CPU usage (nanoseconds)
/sys/fs/cgroup/cpu/cpu.cfs_quota_us          # CPU limit
/sys/fs/cgroup/cpu/cpu.cfs_period_us         # CPU period
/proc/stat                                   # System CPU usage
```

### **Our Previous Method:**
```rust
// We were using /proc/stat only (container's view)
// This doesn't match docker stats calculation
```

## New Implementation: Cgroup-Based

The updated implementation now reads from **the same sources as docker stats**:

### **1. Container CPU Usage (cgroups)**
```rust
// cgroups v1
let usage_total = fs::read_to_string("/sys/fs/cgroup/cpu,cpuacct/cpuacct.usage")?;

// cgroups v2  
let usage_str = fs::read_to_string("/sys/fs/cgroup/cpu.stat")?;
// Parse "usage_usec" field
```

### **2. System CPU Usage (/proc/stat)**
```rust
// Same system CPU calculation as docker stats
let system_usage: u64 = parts[1..8].iter()
    .map(|s| s.parse::<u64>().unwrap_or(0))
    .sum::<u64>() * 10_000_000; // Convert to nanoseconds
```

### **3. CPU Percentage Calculation**
```rust
// Exact same formula as docker stats
let cpu_percent = (container_usage_diff as f64 / system_usage_diff as f64) 
    * online_cpus as f64 * 100.0;
```

## Why This Should Match Docker Stats

### **Same Data Sources:**
- ✅ Container CPU usage from cgroups
- ✅ System CPU usage from /proc/stat  
- ✅ CPU limits from cgroup quota/period
- ✅ Same calculation formula

### **Handles Both cgroups v1 and v2:**
- Automatically detects cgroup version
- Falls back gracefully if cgroup files aren't available
- Respects CPU limits set in docker-compose.yml

### **CPU Limit Awareness:**
```rust
// Reads actual CPU allocation
let quota: i64 = quota_str.trim().parse().unwrap_or(-1);
let period: u64 = period_str.trim().parse().unwrap_or(100000);
let allocated_cpus = (quota as u64 / period).max(1) as u32;
```

## Testing the New Implementation

### **1. Build and Deploy**
```bash
docker compose build --no-cache shippingservice
docker compose up -d shippingservice
```

### **2. Check Logs**
Look for:
```
INFO CPU metrics observable gauges registered successfully (using cgroups like docker stats)
INFO Updated metrics - Container CPU: 95.23% (cgroup-based, matches docker stats), Process CPU: 2.10%, Memory: 45678912 bytes
```

### **3. Compare Side-by-Side**
```bash
# Terminal 1: Monitor docker stats
watch -n 1 'docker stats --no-stream shipping-service'

# Terminal 2: Monitor our metrics
docker logs -f shipping-service | grep "Updated metrics"

# Terminal 3: Generate load
docker exec -it shipping-service bash
yes > /dev/null &
```

### **4. Expected Results**
Both should show similar values:
```
Docker Stats:    CPU: 98.5%
Our Metrics:     Container CPU: 97.8% (cgroup-based, matches docker stats)
```

## Verification Commands

### **Check Cgroup Files**
```bash
# Verify cgroup files exist and are readable
docker exec shipping-service ls -la /sys/fs/cgroup/cpu*
docker exec shipping-service cat /sys/fs/cgroup/cpu,cpuacct/cpuacct.usage
docker exec shipping-service cat /sys/fs/cgroup/cpu/cpu.cfs_quota_us
```

### **Manual Calculation**
```bash
# You can manually verify the calculation
docker exec shipping-service bash -c '
  usage1=$(cat /sys/fs/cgroup/cpu,cpuacct/cpuacct.usage)
  sleep 2
  usage2=$(cat /sys/fs/cgroup/cpu,cpuacct/cpuacct.usage)
  echo "Usage diff: $((usage2 - usage1)) nanoseconds"
'
```

## Troubleshooting

### **If Still Not Matching:**
1. **Check cgroup version** - Some systems use cgroups v2
2. **Verify file permissions** - Container needs read access to cgroup files
3. **Check Docker version** - Older versions may have different cgroup layouts

### **Debug Information:**
The logs now include detailed calculation info:
```
INFO Cgroup CPU calculation - Container: 1234567890 ns, System: 9876543210 ns, CPUs: 1, Usage: 95.23%
```

This new implementation should **exactly match** what you see in `docker stats` because it uses the identical calculation method and data sources!
