use std::fs;

use gameman::bus::Bus;
use gameman::cpu::Cpu;

/// Load a ROM file into the Bus starting at address 0x0000.
///
/// Only loads up to 32KB (0x8000 bytes) into ROM bank 0 area.
#[allow(clippy::cast_possible_truncation)]
fn load_rom(bus: &mut Bus, path: &str) {
    let rom = fs::read(path).expect("Failed to read ROM file");
    for (i, &byte) in rom.iter().enumerate() {
        if i >= 0x8000 {
            break;
        }
        bus.write(i as u16, byte);
    }
}

/// Run a Blargg test ROM and capture serial output.
///
/// Blargg ROMs write output characters to serial port SB (0xFF01),
/// then trigger transfer by writing 0x81 to SC (0xFF02).
/// We intercept these writes to build the output string.
///
/// `max_cycles` prevents infinite loops on unimplemented opcodes.
fn run_blargg_test(rom_path: &str, max_cycles: u64) -> String {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();

    load_rom(&mut bus, rom_path);
    cpu.reset();

    let mut total_cycles = 0u64;
    let mut output = String::new();

    while total_cycles < max_cycles {
        let cycles = u64::from(cpu.step(&mut bus));
        total_cycles += cycles;

        // Check for serial output: SC (0xFF02) == 0x81 triggers transfer.
        // Capture SB (0xFF01) as output character and reset SC.
        if bus.read(0xFF02) == 0x81 {
            let ch = bus.read(0xFF01);
            output.push(ch as char);
            bus.write(0xFF02, 0x00);

            // Stop early if we see a definitive result.
            if output.contains("Passed") || output.contains("Failed") {
                break;
            }
        }
    }

    output
}

/// Run Blargg's `cpu_instrs` test suite.
///
/// This test is ignored by default because it requires most CPU opcodes
/// to be implemented. Enable with: cargo test --ignored
#[test]
#[ignore = "CPU not yet complete enough to run Blargg tests"]
fn test_blargg_cpu_instrs() {
    let output = run_blargg_test("test-roms/cpu_instrs/cpu_instrs.gb", 10_000_000);
    assert!(
        output.contains("Passed"),
        "Expected 'Passed' in output, got: {output}"
    );
}
