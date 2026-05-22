use gameman::cpu::Cpu;

fn main() {
    let mut cpu = Cpu::new();
    cpu.reset();

    println!("Gameman - Gameboy Emulator");
    println!("CPU initialized. PC: 0x{:04X}", cpu.registers.pc);
    println!("SP: 0x{:04X}", cpu.registers.sp);
}
