use std::fmt::Write;
use std::fs;
use std::path::Path;

use gameman::bus::Bus;
use gameman::cpu::Cpu;

/// Load a ROM file into the Bus via the cartridge interface.
fn load_rom(bus: &mut Bus, path: &str) {
    let rom = fs::read(path).expect("Failed to read ROM file");
    bus.load_cartridge(&rom);
}

fn has_terminal_result(output: &str) -> bool {
    output.contains("Passed") || output.contains("Failed")
}

fn test_passed(output: &str) -> bool {
    output.contains("Passed") && !output.contains("Failed")
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
            if has_terminal_result(&output) {
                break;
            }
        }
    }

    output
}

/// Run an individual Blargg test ROM.
fn run_individual_test(rom_path: &str) -> (bool, String) {
    let output = run_blargg_test(rom_path, 100_000_000);
    let passed = test_passed(&output);
    (passed, output)
}

/// Run Blargg's `cpu_instrs` individual test ROMs.
///
/// Skips gracefully when ROMs are not present locally.
/// In CI, ROMs are downloaded and cached by `.github/workflows/ci.yml`.
/// Download manually: `curl -L https://gbdev.gg8.se/files/roms/blargg-gb-tests/cpu_instrs.zip | unzip`
#[test]
fn test_blargg_cpu_instrs() {
    if !Path::new("test-roms/cpu_instrs/individual").is_dir() {
        eprintln!(
            "Skipping: test-roms/cpu_instrs/individual/ not found. \
             Download from https://gbdev.gg8.se/files/roms/blargg-gb-tests/"
        );
        return;
    }
    let tests = [
        "01-special",
        "02-interrupts",
        "03-op sp,hl",
        "04-op r,imm",
        "05-op rp",
        "06-ld r,r",
        "07-jr,jp,call,ret,rst",
        "08-misc instrs",
        "09-op r,r",
        "10-bit ops",
        "11-op a,(hl)",
    ];

    let mut all_passed = true;
    let mut results = String::new();

    for test in &tests {
        let path = format!("test-roms/cpu_instrs/individual/{test}.gb");
        let (passed, output) = run_individual_test(&path);
        if passed {
            let _ = writeln!(results, "{test}: PASSED");
        } else {
            all_passed = false;
            let _ = writeln!(results, "{test}: FAILED - output: {output:?}");
        }
    }

    assert!(all_passed, "Some Blargg tests failed:\n{results}");
}
