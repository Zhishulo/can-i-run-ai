use std::process::Command;

use super::{
    HardwareInfo, GpuInfo, cpu::CpuInfo, detect_sysinfo_base, memory::MemoryInfo, no_window,
};

/// NVIDIA GPUs report name and VRAM reliably via nvidia-smi.
fn query_nvidia_smi() -> Option<(String, f64)> {
    let mut cmd = Command::new("nvidia-smi");
    cmd.args([
        "--query-gpu=name,memory.total",
        "--format=csv,noheader,nounits",
    ]);
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first_line = stdout.lines().next()?.trim().to_string();
    let mut parts = first_line.splitn(2, ',');
    let name = parts.next()?.trim().to_string();
    let mib = parts.next().unwrap_or("").trim().parse::<f64>().ok()?;
    if name.is_empty() {
        return None;
    }
    Some((name, mib / 1024.0))
}

/// Generic adapter name via the CIM/WMI class Win32_VideoController.
/// AdapterRAM is intentionally not read here: it is a 32-bit value and
/// wraps above 4 GB, so it must not be presented as VRAM (see docs).
fn query_video_controller_name() -> Option<String> {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "(Get-CimInstance -ClassName Win32_VideoController | Select-Object -First 1).Name",
    ]);
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub fn detect_windows_hardware() -> HardwareInfo {
    let (cpu, memory, os_name, os_version) = detect_sysinfo_base("Windows");

    let (gpu, has_nvidia) = match query_nvidia_smi() {
        Some((name, vram_gb)) => (
            GpuInfo {
                model: name,
                is_unified_memory: false,
                vram_gb: super::round1(vram_gb),
                metal_support: "CUDA".to_string(),
            },
            true,
        ),
        None => {
            let model = query_video_controller_name().unwrap_or_else(|| "Unknown GPU".to_string());
            (
                GpuInfo {
                    model,
                    is_unified_memory: false,
                    vram_gb: 0.0,
                    metal_support: "DirectX 12".to_string(),
                },
                false,
            )
        }
    };

    let mut backends = Vec::new();
    if has_nvidia {
        backends.push("NVIDIA CUDA".to_string());
    }
    backends.push("CPU inference".to_string());

    HardwareInfo {
        platform: "windows".to_string(),
        arch: std::env::consts::ARCH.to_string(),
        os_name,
        os_version,
        cpu,
        memory,
        gpu,
        backends,
    }
}
