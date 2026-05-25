use super::*;
use crate::bus::Bus;

#[test]
fn test_cpu_new() {
    let cpu = Cpu::new();
    assert_eq!(cpu.registers.pc, 0);
    assert_eq!(cpu.registers.sp, 0);
}

#[test]
fn test_cpu_reset() {
    let mut cpu = Cpu::new();
    cpu.registers.a = 0xFF;
    cpu.registers.pc = 0x5555;

    cpu.reset();

    assert_eq!(cpu.registers.a, 0);
    assert_eq!(cpu.registers.pc, 0x0100);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_nop() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0x00); // NOP

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.pc, 0x0001);
}

#[test]
fn test_ld_r_n() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0x3E); // LD A, n
    bus.write(0x0001, 0x42);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.pc, 0x0002);
}

#[test]
fn test_ld_r_n_boundary() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0x06); // LD B, n
    bus.write(0x0001, 0xFF);

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.b, 0xFF);
}

#[test]
fn test_ld_r_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x37;
    bus.write(0x0000, 0x78); // LD A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0x37);
    assert_eq!(cpu.registers.pc, 0x0001);
}

#[test]
fn test_ld_r_r_all_registers() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();

    // LD A, B
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x78);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x01);

    // LD A, C
    cpu.registers.c = 0x02;
    bus.write(0x0001, 0x79);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x02);

    // LD A, D
    cpu.registers.d = 0x03;
    bus.write(0x0002, 0x7A);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x03);

    // LD A, E
    cpu.registers.e = 0x04;
    bus.write(0x0003, 0x7B);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x04);

    // LD A, H
    cpu.registers.h = 0x05;
    bus.write(0x0004, 0x7C);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x05);

    // LD A, L
    cpu.registers.l = 0x06;
    bus.write(0x0005, 0x7D);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x06);
}

#[test]
fn test_ld_r_r_hl_indirect() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0xC000);
    bus.write(0xC000, 0xAB);
    bus.write(0x0000, 0x7E); // LD A, (HL)

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.a, 0xAB);
}

#[test]
fn test_ld_hl_indirect_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0xCD;
    cpu.registers.set_hl(0xC000);
    bus.write(0x0000, 0x77); // LD (HL), A

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(bus.read(0xC000), 0xCD);
}

#[test]
fn test_inc_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x04); // INC B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.b, 0x02);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
}

#[test]
fn test_inc_r_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0xFF;
    bus.write(0x0000, 0x04); // INC B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.b, 0x00);
    assert!(cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
}

#[test]
fn test_inc_r_half_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x0F;
    bus.write(0x0000, 0x04); // INC B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.b, 0x10);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
}

#[test]
fn test_dec_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x02;
    bus.write(0x0000, 0x05); // DEC B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.b, 0x01);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
}

#[test]
fn test_dec_r_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x05); // DEC B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.b, 0x00);
    assert!(cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
}

#[test]
fn test_dec_r_half_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x10;
    bus.write(0x0000, 0x05); // DEC B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.b, 0x0F);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
}

#[test]
fn test_dec_r_underflow() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x00;
    bus.write(0x0000, 0x05); // DEC B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.b, 0xFF);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
}

#[test]
fn test_add_a_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x01;
    cpu.registers.b = 0x02;
    bus.write(0x0000, 0x80); // ADD A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0x03);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_add_a_r_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x00;
    cpu.registers.b = 0x00;
    bus.write(0x0000, 0x80); // ADD A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
}

#[test]
fn test_add_a_r_half_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x0F;
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x80); // ADD A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x10);
    assert!(cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_add_a_r_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0xFF;
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x80); // ADD A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_add_a_n() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x12;
    bus.write(0x0000, 0xC6); // ADD A, n
    bus.write(0x0001, 0x34);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.a, 0x46);
    assert_eq!(cpu.registers.pc, 0x0002);
}

#[test]
fn test_add_a_hl_indirect() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x20;
    cpu.registers.set_hl(0xC000);
    bus.write(0xC000, 0x20);
    bus.write(0x0000, 0x86); // ADD A, (HL)

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.a, 0x40);
}

#[test]
fn test_inc_hl_indirect() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0xC000);
    bus.write(0xC000, 0x0F);
    bus.write(0x0000, 0x34); // INC (HL)

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(bus.read(0xC000), 0x10);
    assert!(cpu.registers.half_carry());
}

#[test]
fn test_dec_hl_indirect() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0xC000);
    bus.write(0xC000, 0x10);
    bus.write(0x0000, 0x35); // DEC (HL)

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(bus.read(0xC000), 0x0F);
    assert!(cpu.registers.half_carry());
}

#[test]
fn test_sequence_multiple_instructions() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0x06); // LD B, 0x10
    bus.write(0x0001, 0x10);
    bus.write(0x0002, 0x04); // INC B
    bus.write(0x0003, 0x80); // ADD A, B

    cpu.step(&mut bus); // LD B, 0x10
    assert_eq!(cpu.registers.b, 0x10);

    cpu.step(&mut bus); // INC B
    assert_eq!(cpu.registers.b, 0x11);

    cpu.step(&mut bus); // ADD A, B
    assert_eq!(cpu.registers.a, 0x11);
}

#[test]
fn test_inc_rr() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_bc(0x0001);
    bus.write(0x0000, 0x03); // INC BC

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.bc(), 0x0002);
}

#[test]
fn test_inc_rr_overflow() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0xFFFF);
    cpu.registers.set_zero(true);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x23); // INC HL

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x0000);
    // INC rr does not affect any flags.
    assert!(cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_inc_rr_preserves_flags() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_bc(0x0001);
    cpu.registers.set_zero(true);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x03); // INC BC

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.bc(), 0x0002);
    // INC rr does not affect any flags.
    assert!(cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_dec_rr() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_de(0x0002);
    bus.write(0x0000, 0x1B); // DEC DE

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.de(), 0x0001);
}

#[test]
fn test_dec_rr_underflow() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0x0000;
    cpu.registers.set_zero(true);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x3B); // DEC SP

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.sp, 0xFFFF);
    // DEC rr does not affect any flags.
    assert!(cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_dec_rr_preserves_flags() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_bc(0x0001);
    cpu.registers.set_zero(true);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x0B); // DEC BC

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.bc(), 0x0000);
    // DEC rr does not affect any flags.
    assert!(cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_add_hl_rr() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x1234);
    cpu.registers.set_bc(0x5678);
    bus.write(0x0000, 0x09); // ADD HL, BC

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.hl(), 0x68AC);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_add_hl_rr_zero_result_preserves_z() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x0000);
    cpu.registers.set_bc(0x0000);
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0x09); // ADD HL, BC

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x0000);
    assert!(cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_add_hl_rr_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x8000);
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0x29); // ADD HL, HL

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x0000);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_add_hl_rr_half_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x0FFF);
    cpu.registers.set_bc(0x0001);
    bus.write(0x0000, 0x09); // ADD HL, BC

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x1000);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_add_hl_rr_preserves_z() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x0001);
    cpu.registers.set_de(0x0001);
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0x19); // ADD HL, DE

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x0002);
    // ADD HL, rr does not affect Z.
    assert!(cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_add_hl_rr_sp() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x1000);
    cpu.registers.sp = 0x0200;
    bus.write(0x0000, 0x39); // ADD HL, SP

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.hl(), 0x1200);
    assert_eq!(cpu.registers.sp, 0x0200);
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_add_hl_rr_both_h_and_c() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x8FFF);
    bus.write(0x0000, 0x29); // ADD HL, HL

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x1FFE);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_add_hl_rr_sp_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0xF000);
    cpu.registers.sp = 0x1000;
    bus.write(0x0000, 0x39); // ADD HL, SP

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x0000);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_inc_rr_de() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_de(0x1234);
    bus.write(0x0000, 0x13); // INC DE

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.de(), 0x1235);
}

#[test]
fn test_inc_rr_sp() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    bus.write(0x0000, 0x33); // INC SP

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.sp, 0xFFFF);
}

#[test]
fn test_dec_rr_hl() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x0100);
    bus.write(0x0000, 0x2B); // DEC HL

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.hl(), 0x00FF);
}

// ADC tests
#[test]
fn test_adc_a_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x01;
    cpu.registers.b = 0x02;
    bus.write(0x0000, 0x88); // ADC A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0x03);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_adc_a_r_with_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x01;
    cpu.registers.b = 0x02;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x88); // ADC A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x04);
}

#[test]
fn test_adc_a_r_half_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x0F;
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x88); // ADC A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x10);
    assert!(cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_adc_a_r_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0xFF;
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x88); // ADC A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_adc_a_n() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x12;
    bus.write(0x0000, 0xCE); // ADC A, n
    bus.write(0x0001, 0x34);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.a, 0x46);
}

// SUB tests
#[test]
fn test_sub_a_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x05;
    cpu.registers.b = 0x03;
    bus.write(0x0000, 0x90); // SUB A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0x02);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_sub_a_r_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x03;
    cpu.registers.b = 0x03;
    bus.write(0x0000, 0x90); // SUB A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_sub_a_r_half_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x10;
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0x90); // SUB A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x0F);
    assert!(cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_sub_a_r_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x01;
    cpu.registers.b = 0x02;
    bus.write(0x0000, 0x90); // SUB A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0xFF);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_sub_a_n() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x12;
    bus.write(0x0000, 0xD6); // SUB A, n
    bus.write(0x0001, 0x02);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.a, 0x10);
}

// SBC tests
#[test]
fn test_sbc_a_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x05;
    cpu.registers.b = 0x03;
    bus.write(0x0000, 0x98); // SBC A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0x02);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_sbc_a_r_with_carry() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x05;
    cpu.registers.b = 0x03;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x98); // SBC A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x01);
}

#[test]
fn test_sbc_a_r_borrow() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x00;
    cpu.registers.b = 0x01;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x98); // SBC A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0xFE);
    assert!(cpu.registers.carry());
    assert!(cpu.registers.half_carry());
}

// AND tests
#[test]
fn test_and_a_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b1010_1010;
    cpu.registers.b = 0b1100_1100;
    bus.write(0x0000, 0xA0); // AND A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0b1000_1000);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_and_a_r_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b1010_1010;
    cpu.registers.b = 0b0101_0101;
    bus.write(0x0000, 0xA0); // AND A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
}

#[test]
fn test_and_a_n() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0xFF;
    bus.write(0x0000, 0xE6); // AND A, n
    bus.write(0x0001, 0x0F);

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x0F);
}

// XOR tests
#[test]
fn test_xor_a_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b1010_1010;
    cpu.registers.b = 0b1100_1100;
    bus.write(0x0000, 0xA8); // XOR A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0b0110_0110);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_xor_a_r_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x42;
    cpu.registers.b = 0x42;
    bus.write(0x0000, 0xA8); // XOR A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
}

// OR tests
#[test]
fn test_or_a_r() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b1010_1010;
    cpu.registers.b = 0b0101_0101;
    bus.write(0x0000, 0xB0); // OR A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0xFF);
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_or_a_r_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x00;
    cpu.registers.b = 0x00;
    bus.write(0x0000, 0xB0); // OR A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
}

// CP tests
#[test]
fn test_cp_a_r_equal() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x42;
    cpu.registers.b = 0x42;
    bus.write(0x0000, 0xB8); // CP A, B

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 4);
    assert_eq!(cpu.registers.a, 0x42); // A unchanged
    assert!(cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_cp_a_r_less() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x01;
    cpu.registers.b = 0x02;
    bus.write(0x0000, 0xB8); // CP A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x01);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
    assert!(cpu.registers.carry());
}

#[test]
fn test_cp_a_r_greater() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x02;
    cpu.registers.b = 0x01;
    bus.write(0x0000, 0xB8); // CP A, B

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x02);
    assert!(!cpu.registers.zero());
    assert!(cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
    assert!(!cpu.registers.carry());
}

#[test]
fn test_cp_a_n() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x42;
    bus.write(0x0000, 0xFE); // CP A, n
    bus.write(0x0001, 0x42);

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.a, 0x42);
    assert!(cpu.registers.zero());
}

// LD rr, nn tests
#[test]
fn test_ld_rr_nn() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0x01); // LD BC, nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(cpu.registers.bc(), 0x1234);
    assert_eq!(cpu.registers.pc, 0x0003);
}

#[test]
fn test_ld_sp_nn() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0x31); // LD SP, nn
    bus.write(0x0001, 0xFE);
    bus.write(0x0002, 0xFF);

    cpu.step(&mut bus);

    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_ld_hl_indirect_n() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0xC000);
    bus.write(0x0000, 0x36); // LD (HL), n
    bus.write(0x0001, 0xAB);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(bus.read(0xC000), 0xAB);
    assert_eq!(cpu.registers.pc, 0x0002);
}

// PUSH / POP tests
#[test]
fn test_push_pop_bc() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    cpu.registers.set_bc(0x1234);
    bus.write(0x0000, 0xC5); // PUSH BC
    bus.write(0x0001, 0xC1); // POP BC

    cpu.step(&mut bus);
    assert_eq!(cpu.registers.sp, 0xFFFC);
    assert_eq!(bus.read(0xFFFD), 0x12);
    assert_eq!(bus.read(0xFFFC), 0x34);

    cpu.registers.set_bc(0x0000);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.bc(), 0x1234);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_push_pop_af() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    cpu.registers.a = 0x12;
    cpu.registers.set_f(0xF0); // Only upper nibble of F is valid
    bus.write(0x0000, 0xF5); // PUSH AF
    bus.write(0x0001, 0xF1); // POP AF

    cpu.step(&mut bus);
    // F lower nibble should be masked to 0 when pushed
    assert_eq!(bus.read(0xFFFD), 0x12);
    assert_eq!(bus.read(0xFFFC), 0xF0);

    bus.write(0xFFFC, 0xFF); // Corrupt lower nibble on stack
    cpu.registers.a = 0x00;
    cpu.registers.set_f(0x00);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x12);
    assert_eq!(cpu.registers.af() & 0x00FF, 0xF0); // Lower nibble masked on POP
}

// JP tests
#[test]
fn test_jp_nn() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0xC3); // JP nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 16);
    assert_eq!(cpu.registers.pc, 0x1234);
}

#[test]
fn test_jp_nz_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0xC2); // JP NZ, nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 16);
    assert_eq!(cpu.registers.pc, 0x1234);
}

#[test]
fn test_jp_nz_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0xC2); // JP NZ, nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(cpu.registers.pc, 0x0003);
}

#[test]
fn test_jp_z_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0xCA); // JP Z, nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 16);
    assert_eq!(cpu.registers.pc, 0x1234);
}

// JR tests
#[test]
fn test_jr_d_forward() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    bus.write(0x0000, 0x18); // JR d
    bus.write(0x0001, 0x05); // +5

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(cpu.registers.pc, 0x0007); // 0x0002 + 5
}

#[test]
fn test_jr_d_backward() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.pc = 0x0010;
    bus.write(0x0010, 0x18); // JR d
    bus.write(0x0011, 0xFB); // -5

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(cpu.registers.pc, 0x000D); // 0x0012 - 5
}

#[test]
fn test_jr_nz_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0x20); // JR NZ, d
    bus.write(0x0001, 0x05); // +5

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(cpu.registers.pc, 0x0007);
}

#[test]
fn test_jr_nz_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0x20); // JR NZ, d
    bus.write(0x0001, 0x05); // +5

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.pc, 0x0002);
}

// CALL / RET tests
#[test]
fn test_call_ret() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    bus.write(0x0000, 0xCD); // CALL nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);
    bus.write(0x1234, 0xC9); // RET

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 24);
    assert_eq!(cpu.registers.pc, 0x1234);
    assert_eq!(cpu.registers.sp, 0xFFFC);
    assert_eq!(bus.read(0xFFFD), 0x00);
    assert_eq!(bus.read(0xFFFC), 0x03); // Return address

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 16);
    assert_eq!(cpu.registers.pc, 0x0003);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_call_nz_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0xC4); // CALL NZ, nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 24);
    assert_eq!(cpu.registers.pc, 0x1234);
    assert_eq!(cpu.registers.sp, 0xFFFC);
}

#[test]
fn test_call_nz_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0xC4); // CALL NZ, nn
    bus.write(0x0001, 0x34);
    bus.write(0x0002, 0x12);

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 12);
    assert_eq!(cpu.registers.pc, 0x0003);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_ret_nz_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFC;
    cpu.registers.set_zero(false);
    bus.write(0xFFFC, 0x34);
    bus.write(0xFFFD, 0x12);
    bus.write(0x0000, 0xC0); // RET NZ

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 20);
    assert_eq!(cpu.registers.pc, 0x1234);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_ret_nz_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0xC0); // RET NZ

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 8);
    assert_eq!(cpu.registers.pc, 0x0001);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

// RST tests
#[test]
fn test_rst_0() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    bus.write(0x0000, 0xC7); // RST 0x00

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 16);
    assert_eq!(cpu.registers.pc, 0x0000);
    assert_eq!(cpu.registers.sp, 0xFFFC);
    assert_eq!(bus.read(0xFFFD), 0x00);
    assert_eq!(bus.read(0xFFFC), 0x01);
}

#[test]
fn test_rst_38() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    bus.write(0x0000, 0xFF); // RST 0x38

    let cycles = cpu.step(&mut bus);

    assert_eq!(cycles, 16);
    assert_eq!(cpu.registers.pc, 0x0038);
    assert_eq!(cpu.registers.sp, 0xFFFC);
}

#[test]
fn test_call_ret_nested() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;

    // 0x0000: CALL 0x0100
    bus.write(0x0000, 0xCD);
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x01);

    // 0x0100: CALL 0x0200
    bus.write(0x0100, 0xCD);
    bus.write(0x0101, 0x00);
    bus.write(0x0102, 0x02);

    // 0x0200: RET
    bus.write(0x0200, 0xC9);

    // 0x0103: RET
    bus.write(0x0103, 0xC9);

    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0100);
    assert_eq!(cpu.registers.sp, 0xFFFC);

    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0200);
    assert_eq!(cpu.registers.sp, 0xFFFA);

    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0103);
    assert_eq!(cpu.registers.sp, 0xFFFC);

    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0003);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

// --- DAA ---

#[test]
fn test_daa_after_add_no_adjust() {
    // 0x05 + 0x03 = 0x08, BCD digits all < 10, no adjust needed
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x08;
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x27); // DAA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x08);
    assert!(!cpu.registers.carry());
    assert!(!cpu.registers.zero());
}

#[test]
fn test_daa_after_add_low_nibble_overflow() {
    // 0x0A is invalid BCD — DAA should add 0x06
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x0A;
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x27); // DAA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x10);
    assert!(!cpu.registers.carry());
}

#[test]
fn test_daa_after_add_carry() {
    // result > 0x99 sets carry, adds 0x60
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x9A;
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x27); // DAA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.carry());
    assert!(cpu.registers.zero());
}

#[test]
fn test_daa_after_sub_no_adjust() {
    // After subtraction with no borrows, no adjustment
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x05;
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x27); // DAA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x05);
    assert!(!cpu.registers.carry());
}

#[test]
fn test_daa_after_sub_half_carry() {
    // After subtraction with half-carry (borrow from low nibble), subtract 0x06
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x0F; // e.g. result after borrow
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x27); // DAA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x09);
    assert!(!cpu.registers.carry());
}

#[test]
fn test_daa_after_sub_with_carry_no_half_carry() {
    // After subtraction with carry, subtract 0x60
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x40;
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x27); // DAA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0xE0);
    assert!(cpu.registers.carry());
}

#[test]
fn test_daa_after_sub_with_carry_and_half_carry() {
    // After subtraction with both carry and half-carry, subtract 0x66
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x73;
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x27); // DAA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x0D);
    assert!(cpu.registers.carry());
}

// --- Rotates: RLCA, RRCA, RLA, RRA ---

#[test]
fn test_rlca() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b1000_0001;
    bus.write(0x0000, 0x07); // RLCA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0b0000_0011);
    assert!(cpu.registers.carry());
    assert!(!cpu.registers.zero());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
}

#[test]
fn test_rrca() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b0000_0011;
    bus.write(0x0000, 0x0F); // RRCA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0b1000_0001);
    assert!(cpu.registers.carry());
}

#[test]
fn test_rla() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b0100_0000;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x17); // RLA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0b1000_0001);
    assert!(!cpu.registers.carry());
}

#[test]
fn test_rra() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b0000_0010;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x1F); // RRA
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0b1000_0001);
    assert!(!cpu.registers.carry());
}

// --- CPL, SCF, CCF ---

#[test]
fn test_cpl() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0b1010_0101;
    bus.write(0x0000, 0x2F); // CPL
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0b0101_1010);
    assert!(cpu.registers.subtract());
    assert!(cpu.registers.half_carry());
}

#[test]
fn test_scf() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(false);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    bus.write(0x0000, 0x37); // SCF
    cpu.step(&mut bus);
    assert!(cpu.registers.carry());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
}

#[test]
fn test_ccf_set_to_clear() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x3F); // CCF
    cpu.step(&mut bus);
    assert!(!cpu.registers.carry());
    assert!(!cpu.registers.subtract());
    assert!(!cpu.registers.half_carry());
}

#[test]
fn test_ccf_clear_to_set() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x3F); // CCF
    cpu.step(&mut bus);
    assert!(cpu.registers.carry());
}

// --- Conditional control flow: Z, NC, C ---

#[test]
fn test_jp_z_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0xCA); // JP Z, nn
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x10);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0003);
}

#[test]
fn test_jp_nc_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0xD2); // JP NC, nn
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x10);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x1000);
}

#[test]
fn test_jp_nc_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0xD2); // JP NC, nn
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x10);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0003);
}

#[test]
fn test_jp_c_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0xDA); // JP C, nn
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x10);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x1000);
}

#[test]
fn test_jp_c_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0xDA); // JP C, nn
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x10);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0003);
}

#[test]
fn test_jr_z_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0x28); // JR Z, e
    bus.write(0x0001, 0x05);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0007);
}

#[test]
fn test_jr_z_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0x28); // JR Z, e
    bus.write(0x0001, 0x05);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0002);
}

#[test]
fn test_jr_nc_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x30); // JR NC, e
    bus.write(0x0001, 0x04);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0006);
}

#[test]
fn test_jr_c_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0x38); // JR C, e
    bus.write(0x0001, 0x04);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0006);
}

#[test]
fn test_jr_c_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0x38); // JR C, e
    bus.write(0x0001, 0x04);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0002);
}

#[test]
fn test_call_z_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    cpu.registers.set_zero(true);
    bus.write(0x0000, 0xCC); // CALL Z, nn
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x05);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0500);
    assert_eq!(cpu.registers.sp, 0xFFFC);
}

#[test]
fn test_call_z_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFE;
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0xCC); // CALL Z, nn
    bus.write(0x0001, 0x00);
    bus.write(0x0002, 0x05);
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0003);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_ret_z_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFC;
    cpu.registers.set_zero(true);
    bus.write(0xFFFC, 0x00);
    bus.write(0xFFFD, 0x05);
    bus.write(0x0000, 0xC8); // RET Z
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0500);
    assert_eq!(cpu.registers.sp, 0xFFFE);
}

#[test]
fn test_ret_z_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFC;
    cpu.registers.set_zero(false);
    bus.write(0x0000, 0xC8); // RET Z
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0001);
    assert_eq!(cpu.registers.sp, 0xFFFC);
}

#[test]
fn test_ret_nc_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFC;
    cpu.registers.set_carry(false);
    bus.write(0xFFFC, 0x34);
    bus.write(0xFFFD, 0x12);
    bus.write(0x0000, 0xD0); // RET NC
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x1234);
}

#[test]
fn test_ret_nc_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFC;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0xD0); // RET NC
    let cycles = cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0001);
    assert_eq!(cpu.registers.sp, 0xFFFC);
    assert_eq!(cycles, 8);
}

#[test]
fn test_ret_c_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFC;
    cpu.registers.set_carry(true);
    bus.write(0xFFFC, 0x34);
    bus.write(0xFFFD, 0x12);
    bus.write(0x0000, 0xD8); // RET C
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x1234);
}

#[test]
fn test_ret_c_not_taken() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.sp = 0xFFFC;
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0xD8); // RET C
    let cycles = cpu.step(&mut bus);
    assert_eq!(cpu.registers.pc, 0x0001);
    assert_eq!(cpu.registers.sp, 0xFFFC);
    assert_eq!(cycles, 8);
}

// --- CB prefix ---

#[test]
fn test_cb_rlc_b() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0b1000_0001;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x00); // RLC B
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.b, 0b0000_0011);
    assert!(cpu.registers.carry());
    assert!(!cpu.registers.zero());
}

#[test]
fn test_cb_rrc_c() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.c = 0b0000_0011;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x09); // RRC C
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.c, 0b1000_0001);
    assert!(cpu.registers.carry());
}

#[test]
fn test_cb_rl_d() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.d = 0b0100_0000;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x12); // RL D
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.d, 0b1000_0001);
    assert!(!cpu.registers.carry());
}

#[test]
fn test_cb_rr_e() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.e = 0b0000_0010;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x1B); // RR E
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.e, 0b1000_0001);
    assert!(!cpu.registers.carry());
}

#[test]
fn test_cb_sla_h() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.h = 0b1100_0000;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x24); // SLA H
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.h, 0b1000_0000);
    assert!(cpu.registers.carry());
    assert!(!cpu.registers.zero());
}

#[test]
fn test_cb_sra_l() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.l = 0b1000_0010;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x2D); // SRA L
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.l, 0b1100_0001);
    assert!(!cpu.registers.carry());
}

#[test]
fn test_cb_swap_a() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0xAB;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x37); // SWAP A
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0xBA);
    assert!(!cpu.registers.zero());
}

#[test]
fn test_cb_swap_zero() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.a = 0x00;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x37); // SWAP A
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.zero());
}

#[test]
fn test_cb_srl_b() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0b1000_0001;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x38); // SRL B
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.b, 0b0100_0000);
    assert!(cpu.registers.carry());
}

#[test]
fn test_cb_bit_set() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0b0000_0100;
    cpu.registers.set_carry(true);
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x50); // BIT 2, B
    cpu.step(&mut bus);
    assert!(!cpu.registers.zero()); // bit is set → Z=0
    assert!(cpu.registers.half_carry());
    assert!(!cpu.registers.subtract());
    assert!(cpu.registers.carry()); // BIT preserves carry
}

#[test]
fn test_cb_bit_clear() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0b0000_0000;
    cpu.registers.set_carry(false);
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x50); // BIT 2, B
    cpu.step(&mut bus);
    assert!(cpu.registers.zero()); // bit is 0 → Z=1
    assert!(!cpu.registers.carry()); // BIT preserves carry
}

#[test]
fn test_cb_res_bit() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0xFF;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x80); // RES 0, B
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.b, 0xFE);
}

#[test]
fn test_cb_set_bit() {
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.b = 0x00;
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0xC0); // SET 0, B
    cpu.step(&mut bus);
    assert_eq!(cpu.registers.b, 0x01);
}

#[test]
fn test_cb_hl_indirect() {
    // BIT via (HL) — verify it reads from memory
    let mut cpu = Cpu::new();
    let mut bus = Bus::new();
    cpu.registers.set_hl(0x8000);
    bus.write(0x8000, 0b0000_1000);
    bus.write(0x0000, 0xCB);
    bus.write(0x0001, 0x5E); // BIT 3, (HL)
    cpu.step(&mut bus);
    assert!(!cpu.registers.zero()); // bit 3 is set
}
