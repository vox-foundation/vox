use sysinfo::System;

#[test]
fn test_probe_apple_silicon_memory_invariants() {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let mut sys = System::new();
        sys.refresh_memory();
        let total_ram_mb = sys.total_memory() / (1024 * 1024);
        assert!(total_ram_mb > 0, "RAM must be detected");
        let ceiling_75 = (total_ram_mb * 3) / 4;
        let headroom = total_ram_mb - ceiling_75;
        assert!(headroom >= 2048, "Must preserve >= 2GB headroom");
    }
}
