use gameman::bus::Bus;
use gameman::cpu::Cpu;

fn make_boot_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x100];
    // Minimal boot ROM: sets A=0x01, then disables boot ROM via LDH [$FF50], A.
    // The LDH instruction sits at 0xFE so PC naturally reaches 0x100 after execution.
    rom[0x00] = 0x3E; // LD A, n
    rom[0x01] = 0x01; // n = 0x01
    rom[0xFE] = 0xE0; // LDH [n], A
    rom[0xFF] = 0x50; // n = 0x50 (address 0xFF50)
    rom
}

fn make_cartridge() -> Vec<u8> {
    let mut data = vec![0u8; 0x8000]; // 32KB, 2 banks
    data[0x0147] = 0x01; // MBC1
    data[0x0148] = 0; // 32KB
    data[0x0149] = 0;
    // Put a marker at the cartridge entry point 0x0100
    data[0x0100] = 0x00; // NOP
    data[0x0101] = 0x76; // HALT
    // Put a marker at 0x0000 (cartridge bank 0)
    data[0x0000] = 0xC3; // JP nn — should NOT be visible while boot ROM active
    data
}

#[test]
fn test_boot_rom_overlays_cartridge_reads() {
    let boot = make_boot_rom();
    let cart = make_cartridge();

    let mut bus = Bus::new();
    bus.load_cartridge(&cart);
    bus.load_boot_rom(&boot);

    // Boot ROM data should be visible at 0x0000-0x00FF
    assert_eq!(bus.read(0x0000), 0x3E); // LD A, n
    assert_eq!(bus.read(0x0001), 0x01);
    assert_eq!(bus.read(0xFE), 0xE0); // LDH [n], A
    assert_eq!(bus.read(0xFF), 0x50);

    // Cartridge data should be visible at 0x0100+
    assert_eq!(bus.read(0x0100), 0x00); // NOP
    assert_eq!(bus.read(0x0101), 0x76); // HALT
}

#[test]
fn test_boot_rom_disable_on_ff50_write() {
    let boot = make_boot_rom();
    let cart = make_cartridge();

    let mut bus = Bus::new();
    bus.load_cartridge(&cart);
    bus.load_boot_rom(&boot);
    assert!(bus.has_boot_rom());

    // Disable boot ROM by writing to 0xFF50
    bus.write(0xFF50, 0x01);
    assert!(!bus.has_boot_rom());

    // Now 0x0000-0x00FF should return cartridge bank 0 data
    assert_eq!(bus.read(0x0000), 0xC3); // JP nn from cartridge
}

#[test]
fn test_boot_rom_write_to_0000_ignored_while_active() {
    let boot = make_boot_rom();
    let cart = make_cartridge();

    let mut bus = Bus::new();
    bus.load_cartridge(&cart);
    bus.load_boot_rom(&boot);

    // Write to 0x0000 while boot ROM is active — should be ignored
    bus.write(0x0000, 0xFF);
    assert_eq!(bus.read(0x0000), 0x3E); // Boot ROM still intact
}

#[test]
fn test_boot_rom_mbc_register_writes_blocked_while_active() {
    let boot = make_boot_rom();
    let mut cart_data = vec![0u8; 0x8000];
    cart_data[0x0147] = 0x01; // MBC1
    cart_data[0x0148] = 0;
    cart_data[0x0149] = 0;

    let mut bus = Bus::new();
    bus.load_cartridge(&cart_data);
    bus.load_boot_rom(&boot);

    // Write to 0x0000 (MBC1 RAM enable register) while boot ROM active — should be ignored.
    bus.write(0x0000, 0x0A);
    // External RAM should still be disabled (returns 0xFF).
    assert_eq!(bus.read(0xA000), 0xFF);
}

#[test]
fn test_boot_rom_read_ff50_returns_ff() {
    let boot = make_boot_rom();
    let mut bus = Bus::new();
    bus.load_boot_rom(&boot);

    assert_eq!(bus.read(0xFF50), 0xFF);

    // After disabling
    bus.write(0xFF50, 0x01);
    assert_eq!(bus.read(0xFF50), 0xFF);
}

#[test]
fn test_boot_rom_disable_is_one_way_latch() {
    let boot = make_boot_rom();
    let cart = make_cartridge();

    let mut bus = Bus::new();
    bus.load_cartridge(&cart);
    bus.load_boot_rom(&boot);

    bus.write(0xFF50, 0x01);
    assert!(!bus.has_boot_rom());

    // Writing again doesn't re-enable
    bus.write(0xFF50, 0x00);
    assert!(!bus.has_boot_rom());
    // Cartridge data still visible
    assert_eq!(bus.read(0x0000), 0xC3);
}

#[test]
fn test_no_boot_rom_backward_compatible() {
    let cart = make_cartridge();
    let mut bus = Bus::new();
    bus.load_cartridge(&cart);

    // Without boot ROM, 0x0000 goes straight to cartridge
    assert_eq!(bus.read(0x0000), 0xC3);
    assert!(!bus.has_boot_rom());
}

#[test]
fn test_reset_with_boot_rom_sets_pre_boot_state() {
    let mut cpu = Cpu::new();
    // Set some non-zero state to verify reset clears it.
    cpu.registers.pc = 0x1234;
    cpu.registers.sp = 0xFFFE;
    cpu.registers.a = 0x42;
    cpu.reset_with_boot_rom();

    // PC starts at 0x0000 (boot ROM entry point).
    assert_eq!(cpu.registers.pc, 0x0000);
    // SP is zeroed (on real hardware this is undefined, but zeroed for determinism).
    assert_eq!(cpu.registers.sp, 0x0000);
    // GP registers are zeroed for deterministic testing.
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.af(), 0x0000);
    assert_eq!(cpu.registers.b, 0x00);
    assert_eq!(cpu.registers.c, 0x00);
    assert_eq!(cpu.registers.d, 0x00);
    assert_eq!(cpu.registers.e, 0x00);
    assert_eq!(cpu.registers.h, 0x00);
    assert_eq!(cpu.registers.l, 0x00);
}

#[test]
fn test_boot_to_cartridge_handoff() {
    let boot = make_boot_rom();
    let cart = make_cartridge();

    let mut bus = Bus::new();
    bus.load_cartridge(&cart);
    bus.load_boot_rom(&boot);

    let mut cpu = Cpu::new();
    cpu.reset_with_boot_rom();

    // Execute LD A, 0x01 at 0x00 (8 cycles)
    let c1 = cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x01);
    assert_eq!(cpu.registers.pc, 0x02);
    assert!(bus.has_boot_rom());

    // Skip to 0xFE where LDH [0xFF50], A lives
    cpu.registers.pc = 0xFE;

    // Execute LDH [0xFF50], A at 0xFE (12 cycles) — disables boot ROM
    let c2 = cpu.step(&mut bus);
    assert!(
        !bus.has_boot_rom(),
        "boot ROM should be disabled after LDH [FF50]"
    );

    // PC should now be at 0x100 (cartridge entry point)
    assert_eq!(cpu.registers.pc, 0x0100);

    // Next instruction is NOP from cartridge
    let c3 = cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0101);

    assert!(c1 > 0 && c2 > 0 && c3 > 0);
}

#[test]
fn test_load_boot_rom_panics_on_wrong_size() {
    let mut bus = Bus::new();
    let result = std::panic::catch_unwind(move || {
        bus.load_boot_rom(&[0u8; 128]);
    });
    assert!(result.is_err());
}
