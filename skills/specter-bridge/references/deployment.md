# Tuquet Network Bridge: Local Deployment & Diagnostics

## Local Deployment Runbook (`deploy local bridge`)

When deploying or rebuilding the bridge service locally on Windows:

```powershell
# Step 1: Release Win32 binary locks
specter bridge stop

# Step 2: Build and install latest CLI binary
cargo install --path cli/

# Step 3: Relaunch Bridge with required tunnels
specter bridge start --ssh

# Step 4: Smoke Test & Verify Status
specter bridge status
```

---

## Low-Level Diagnostics

```powershell
# 1. Verify local TCP port listeners (2222, 1080, 8118)
Get-NetTCPConnection -LocalPort 2222,1080,8118 -State Listen -ErrorAction SilentlyContinue

# 2. Test SOCKS5 proxy egress IP
curl.exe -s -x socks5h://127.0.0.1:1080 https://api.ipify.org?format=json

# 3. Test HTTP adapter egress IP
curl.exe -s -x http://127.0.0.1:8118 https://api.ipify.org?format=json

# 4. Non-interactive SSH test via local port 2222
ssh -o BatchMode=yes -o StrictHostKeyChecking=no -o ConnectTimeout=5 -p 2222 root@127.0.0.1 "uname -a"
```
