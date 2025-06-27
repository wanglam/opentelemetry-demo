# CPU Metrics Testing Guide - Single Core Container

## Problem Analysis

You're absolutely correct! The issue is **multi-core CPU allocation**. Here's what's happening:

### **Multi-Core Problem**
```
Host: 4 CPU cores available
Container: Has access to all 4 cores
yes > /dev/null: Uses 100% of 1 core
Result: 100% ÷ 4 cores = 25% overall CPU usage
```

This is why your metric shows 25% instead of 100%.

## Solution: Limit Container to 1 CPU Core

### **Method 1: Updated docker-compose.yml (Recommended)**

I've updated your main docker-compose.yml:
```yaml
shippingservice:
  deploy:
    resources:
      limits:
        memory: 250M
        cpus: '1.0'  # Limit to 1 CPU core
```

### **Method 2: Alternative Configuration**

For testing, use the dedicated CPU-limited configuration:
```bash
# Use the CPU-limited compose file
docker-compose -f docker-compose-cpu-limited.yml up shippingservice
```

### **Method 3: Docker Run Command (For Quick Testing)**
```bash
# Build the image first
docker build -t shipping-test ./src/shippingservice/

# Run with CPU limits
docker run -d \
  --name shipping-cpu-test \
  --cpus="1.0" \
  --memory="250m" \
  -p 50051:50051 \
  -e SHIPPING_SERVICE_PORT=50051 \
  -e OTEL_EXPORTER_OTLP_ENDPOINT=http://host.docker.internal:4317 \
  shipping-test
```

## Testing Steps

### **1. Deploy with CPU Limits**
```bash
# Option A: Use updated main compose file
docker compose down shippingservice
docker compose build --no-cache shippingservice
docker compose up -d shippingservice

# Option B: Use dedicated CPU-limited file
docker-compose -f docker-compose-cpu-limited.yml up -d shippingservice
```

### **2. Verify CPU Limits**
```bash
# Check container resource limits
docker inspect shipping-service | grep -A 10 "Resources"

# Should show:
# "CpuQuota": 100000,  # 1.0 CPU
# "CpuPeriod": 100000
```

### **3. Test CPU Usage**
```bash
# Enter the container
docker exec -it shipping-service bash

# Check CPU cores visible to container
nproc  # Should show 1 (or limited number)

# Generate 100% CPU load
yes > /dev/null &

# In another terminal, check docker stats
docker stats shipping-service
# Should show close to 100% CPU usage
```

### **4. Monitor Metrics**
```bash
# Check service logs
docker logs shipping-service | grep "Updated metrics"

# Should now show:
# "Container CPU: 95-100% (accurate)"
```

## Expected Results

### **Before CPU Limits (Multi-core)**
- `yes > /dev/null` → 25% CPU usage (100% ÷ 4 cores)
- Metric shows inaccurate low values

### **After CPU Limits (Single core)**
- `yes > /dev/null` → ~100% CPU usage
- Metric shows accurate high values
- Matches `docker stats` output

## Additional CPU Limit Options

### **Fractional CPU Limits**
```yaml
# Allow 0.5 CPU cores (50% of one core max)
cpus: '0.5'

# Allow 2.5 CPU cores
cpus: '2.5'
```

### **CPU Affinity (Pin to specific cores)**
```yaml
# Pin to specific CPU cores
cpuset: "0,1"  # Use only cores 0 and 1
```

### **CPU Shares (Relative priority)**
```yaml
# Lower priority when competing for CPU
cpu_shares: 512  # Default is 1024
```

## Verification Commands

### **Check Container CPU Configuration**
```bash
# View container resource limits
docker inspect shipping-service | jq '.HostConfig.Resources'

# Check CPU quota and period
docker exec shipping-service cat /sys/fs/cgroup/cpu/cpu.cfs_quota_us
docker exec shipping-service cat /sys/fs/cgroup/cpu/cpu.cfs_period_us
```

### **Monitor Real-time CPU Usage**
```bash
# Docker stats (should match your metrics)
docker stats --no-stream shipping-service

# Inside container
docker exec shipping-service top
docker exec shipping-service htop  # if available
```

### **Test Different Load Levels**
```bash
# 100% CPU load
yes > /dev/null &

# 50% CPU load (approximate)
yes > /dev/null & yes > /dev/null & sleep 1; killall yes &

# Multiple processes (should still max at 100% with 1 CPU limit)
yes > /dev/null & yes > /dev/null & yes > /dev/null &
```

## Troubleshooting

### **If CPU limits don't work:**
1. **Check Docker version** - Older versions may not support all limit types
2. **Try alternative syntax** - Use the docker-compose-cpu-limited.yml file
3. **Verify cgroups** - Ensure cgroups v1 or v2 are properly configured

### **If metrics still seem off:**
1. **Check logs for debug info** - Look for CPU calculation details
2. **Compare with docker stats** - Should match closely
3. **Test with different load patterns** - Try various CPU-intensive tasks

With CPU limits in place, your `yes > /dev/null` test should now show close to 100% CPU usage in both the metrics and `docker stats`!
