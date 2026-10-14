"""Read-only Pi metrics; no media names, capabilities, secrets or NAS writes.
Execute via the approved SSH session on the Pi; JSON lines remain on stdout.
CPU is interval process CPU as a percentage of ONE core (not ps lifetime CPU).
Network counts the whole wlan0 interface including CIFS and other services.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--seconds",type=int,default=1800)
parser.add_argument("--interval",type=int,default=30)
parser.add_argument("--phase",choices=["idle","two","three"],required=True)
args=parser.parse_args()
assert 1<=args.seconds<=3600 and 1<=args.interval<=60
hz=os.sysconf("SC_CLK_TCK")
page=os.sysconf("SC_PAGE_SIZE")
start=time.monotonic()
last=None
samples=[]
while True:
    now=time.monotonic()
    pid=int(subprocess.check_output(["systemctl","show","chillcenter.service","-p","MainPID","--value"],text=True).strip())
    if not pid:
        raise RuntimeError("Chill Center has no running PID during resource measurement")
    data=Path(f"/proc/{pid}/stat").read_text().rsplit(") ",1)[1].split()
    ticks=int(data[11])+int(data[12])
    rss=int(data[21])*page
    mem={line.split(":",1)[0]:int(line.split()[1]) for line in Path("/proc/meminfo").read_text().splitlines() if line.split()[1].isdigit()}
    nic=next(line.split(":",1)[1].split() for line in Path("/proc/net/dev").read_text().splitlines() if line.strip().startswith("wlan0:"))
    rx,tx=int(nic[0]),int(nic[8])
    cpu=rx_mbps=tx_mbps=None
    if last is not None:
        assert pid==last[0], "service PID changed"
        elapsed=now-last[1]
        cpu=100.*(ticks-last[2])/hz/elapsed
        rx_mbps=8.*(rx-last[3])/elapsed/1_000_000.
        tx_mbps=8.*(tx-last[4])/elapsed/1_000_000.
    last=(pid,now,ticks,rx,tx)
    temp=float(Path("/sys/class/thermal/thermal_zone0/temp").read_text())/1000.
    row={"phase":args.phase,"elapsed":round(now-start,2),"pid":pid,"rss_mib":round(rss/1048576.,2),"cpu_one_core_pct":None if cpu is None else round(cpu,2),"mem_available_mib":round(mem["MemAvailable"]/1024.,2),"swap_used_mib":round((mem["SwapTotal"]-mem["SwapFree"])/1024.,2),"temperature_c":temp,"wlan_rx_mbps":None if rx_mbps is None else round(rx_mbps,3),"wlan_tx_mbps":None if tx_mbps is None else round(tx_mbps,3)}
    samples.append(row)
    print("RESOURCE "+json.dumps(row),flush=True)
    if now-start>=args.seconds:
        summary={"phase":args.phase,"seconds":round(now-start,2),"samples":len(samples),"max_rss_mib":max(r["rss_mib"] for r in samples),"max_cpu_one_core_pct":max((r["cpu_one_core_pct"] or 0) for r in samples),"min_available_mib":min(r["mem_available_mib"] for r in samples),"max_temperature_c":max(r["temperature_c"] for r in samples),"max_swap_used_mib":max(r["swap_used_mib"] for r in samples),"max_wlan_rx_mbps":max((r["wlan_rx_mbps"] or 0) for r in samples),"max_wlan_tx_mbps":max((r["wlan_tx_mbps"] or 0) for r in samples),"mean_cpu_one_core_pct":sum((r["cpu_one_core_pct"] or 0) for r in samples[1:])/max(1,len(samples)-1)}
        print("RESOURCE_SUMMARY "+json.dumps(summary),flush=True)
        break
    time.sleep(min(args.interval,args.seconds-(now-start)))
