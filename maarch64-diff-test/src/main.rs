use unicorn_engine::{RegisterARM64, Unicorn, Arch, Mode, Prot};
use maarch64_core::{cpu::CpuContext, interp::Interpreter, memory::MemoryManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("[+] Initializing Maarch64 Differential Test Harness against Unicorn Engine Oracle...");

    let code_addr: u64 = 0x400000;
    let mem_size: usize = 0x100000; // 1MB

    // 1. Initialize Unicorn Engine
    let mut uc = Unicorn::new(Arch::ARM64, Mode::ARM)?;
    uc.mem_map(code_addr, mem_size as u64, Prot::ALL)?;

    // 2. Initialize Maarch64 Memory & CpuContext
    let mut mem = MemoryManager::new();
    mem.map_anonymous(code_addr, mem_size)?;
    let mut ctx = CpuContext::new();
    ctx.pc = code_addr;

    // Test AArch64 Instruction Stream:
    // 0x00: MOV X0, #1          (0xd2800020)
    // 0x04: MOV X1, #10         (0xd2800141)
    // 0x08: ADD X2, X0, X1      (0x8b010002)
    // 0x0c: SUB X3, X2, X0      (0xcb000043)
    let code: [u8; 16] = [
        0x20, 0x00, 0x80, 0xd2, // MOV X0, #1
        0x41, 0x01, 0x80, 0xd2, // MOV X1, #10
        0x02, 0x00, 0x01, 0x8b, // ADD X2, X0, X1
        0x43, 0x00, 0x00, 0xcb, // SUB X3, X2, X0
    ];

    uc.mem_write(code_addr, &code)?;
    mem.write(code_addr, &code)?;

    let regs_to_check = [
        (0, RegisterARM64::X0, "X0"),
        (1, RegisterARM64::X1, "X1"),
        (2, RegisterARM64::X2, "X2"),
        (3, RegisterARM64::X3, "X3"),
    ];

    println!("[+] Running step-by-step differential verification (4 instructions)...");

    for step in 0..4 {
        let pc_before = ctx.pc;
        
        // Step in Unicorn Engine
        uc.emu_start(pc_before, pc_before + 4, 0, 1)?;
        
        // Step in Maarch64 Interpreter
        Interpreter::step(&mut ctx, &mut mem)?;

        println!("[Step {}] PC = {:#x} -> {:#x}", step + 1, pc_before, ctx.pc);

        // State Comparison
        for &(reg_idx, uc_reg, reg_name) in &regs_to_check {
            let maarch_val = ctx.get_x(reg_idx);
            let uc_val = uc.reg_read(uc_reg)?;

            if maarch_val != uc_val {
                eprintln!(
                    "[-] DIFFERENCE DETECTED at Step {} (PC={:#x}) for register {}!",
                    step + 1,
                    pc_before,
                    reg_name
                );
                eprintln!("    Maarch64: {:#x}", maarch_val);
                eprintln!("    Unicorn:  {:#x}", uc_val);
                return Err("Differential verification state mismatch!".into());
            } else {
                println!("    {} matches: {:#x}", reg_name, maarch_val);
            }
        }
    }

    println!("[+] ALL 4 INSTRUCTIONS PASSED BIT-EXACT DIFFERENTIAL VERIFICATION WITH UNICORN ORACLE!");
    Ok(())
}
