//! Smoke test against the real machine this test runs on.
//!
//! Ignored by default (CI runners have no meaningful GPU); run it on
//! a real desktop with:
//!
//! ```text
//! cargo test --test real_machine -- --ignored --nocapture
//! ```

use can_i_run_ai::hardware;

#[test]
#[ignore = "requires a real desktop machine with GPU drivers"]
fn real_machine_hardware_smoke() {
    let hw = hardware::detect_hardware();
    println!(
        "{}",
        serde_json::to_string_pretty(&hw).expect("HardwareInfo serializes")
    );

    assert!(!hw.cpu.model.is_empty(), "CPU model must be detected");
    assert!(hw.memory.total_bytes > 0, "total RAM must be detected");
    assert!(
        hw.memory.free_bytes > 0,
        "free RAM must be detected (no fake zeros)"
    );
    assert!(!hw.gpu.model.is_empty(), "GPU model must be present");
    if hw.platform == "windows" && hw.gpu.vendor == "nvidia" {
        assert!(hw.gpu.vram_gb > 0.0, "NVML must report VRAM");
        assert!(hw.gpu.driver_version.is_some(), "NVML must report driver");
    }
}
