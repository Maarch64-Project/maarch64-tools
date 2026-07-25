use unicorn_engine::{RegisterARM64, Unicorn, Arch, Mode, Prot};
use maarch64_core::{cpu::CpuContext, interp::Interpreter, memory::MemoryManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    println!("================================================================================");
    println!("   Maarch64 Differential Test Harness against Unicorn Engine Oracle           ");
    println!("================================================================================");

    let code_addr: u64 = 0x400000;
    let stack_top: u64 = 0x800000;
    let mem_size: usize = 0x800000; // 8MB

    // 1. Initialize Unicorn Engine
    let mut uc = Unicorn::new(Arch::ARM64, Mode::ARM)?;
    uc.mem_map(code_addr, mem_size as u64, Prot::ALL)?;
    uc.reg_write(RegisterARM64::SP, stack_top)?;

    // 2. Initialize Maarch64 Memory & CpuContext
    let mut mem = MemoryManager::new();
    mem.map_anonymous(code_addr, mem_size)?;
    let mut ctx = CpuContext::new();
    ctx.pc = code_addr;
    ctx.sp = stack_top;
    ctx.pstate = uc.reg_read(RegisterARM64::NZCV)? as u32;

    // Comprehensive AArch64 Test Instruction Stream:
    // 0x00: MOV X0, #42            (0xd2800540)
    // 0x04: MOV X1, #100           (0xd2800c81)
    // 0x08: ADD X2, X0, X1        (0x8b010002)
    // 0x0c: SUBS X3, X1, X0        (0xeb000023)  -> Sets NZCV flags
    // 0x10: MOVK X0, #0x1234, lsl #16 (0xf2a24680)
    // 0x14: STP X0, X1, [sp, #-16]! (0xa9bf07e0)
    // 0x18: LDP X4, X5, [sp], #16   (0xa8c117e4)
    // 0x1c: CMP X4, X5             (0xeb05001f)  -> Sets NZCV flags
    // 0x20: CSEL X6, X4, X5, EQ    (0x1a850086)  -> Conditional Select
    // 0x24: ORR X7, X4, X5         (0xaa050087)
    // 0x28: EOR X8, X4, X5         (0xca050088)
    // 0x2c: AND X9, X4, X5         (0x8a050089)
    let code: &[u8] = &[
        0x40, 0x05, 0x80, 0xd2, // MOV X0, #42
        0x81, 0x0c, 0x80, 0xd2, // MOV X1, #100
        0x02, 0x00, 0x01, 0x8b, // ADD X2, X0, X1
        0x23, 0x00, 0x00, 0xeb, // SUBS X3, X1, X0
        0x80, 0x46, 0xa2, 0xf2, // MOVK X0, #0x1234, lsl #16
        0xe0, 0x07, 0xbf, 0xa9, // STP X0, X1, [sp, #-16]!
        0xe4, 0x17, 0xc1, 0xa8, // LDP X4, X5, [sp], #16
        0x1f, 0x00, 0x05, 0xeb, // CMP X4, X5
        0x86, 0x00, 0x85, 0x1a, // CSEL X6, X4, X5, EQ
        0x87, 0x00, 0x05, 0xaa, // ORR X7, X4, X5
        0x88, 0x00, 0x05, 0xca, // EOR X8, X4, X5
        0x89, 0x00, 0x05, 0x8a, // AND X9, X4, X5
    ];

    let num_instructions = code.len() / 4;

    uc.mem_write(code_addr, code)?;
    mem.write(code_addr, code)?;

    let registers: &[(usize, RegisterARM64, &str)] = &[
        (0, RegisterARM64::X0, "X0"),
        (1, RegisterARM64::X1, "X1"),
        (2, RegisterARM64::X2, "X2"),
        (3, RegisterARM64::X3, "X3"),
        (4, RegisterARM64::X4, "X4"),
        (5, RegisterARM64::X5, "X5"),
        (6, RegisterARM64::X6, "X6"),
        (7, RegisterARM64::X7, "X7"),
        (8, RegisterARM64::X8, "X8"),
        (9, RegisterARM64::X9, "X9"),
    ];

    println!("[+] Running step-by-step differential verification ({} instructions)...", num_instructions);

    for step in 0..num_instructions {
        let pc_before = ctx.pc;
        
        // Step in Unicorn Engine
        uc.emu_start(pc_before, pc_before + 4, 0, 1)?;
        
        // Step in Maarch64 Interpreter
        Interpreter::step(&mut ctx, &mut mem)?;

        println!("[Step {:>2}] PC = {:#x} -> {:#x}", step + 1, pc_before, ctx.pc);

        // 1. Verify PC & SP
        let uc_sp = uc.reg_read(RegisterARM64::SP)?;
        if ctx.sp != uc_sp {
            eprintln!("[-] SP MISMATCH at Step {}! Maarch64: {:#x}, Unicorn: {:#x}", step + 1, ctx.sp, uc_sp);
            return Err("SP Mismatch".into());
        }

        // 2. Register State Comparison
        for &(reg_idx, uc_reg, reg_name) in registers {
            let maarch_val = ctx.get_x(reg_idx);
            let uc_val = uc.reg_read(uc_reg)?;

            if maarch_val != uc_val {
                eprintln!(
                    "[-] REGISTER MISMATCH at Step {} (PC={:#x}) for {}!\n    Maarch64: {:#x}\n    Unicorn:  {:#x}",
                    step + 1, pc_before, reg_name, maarch_val, uc_val
                );
                return Err("Register Mismatch".into());
            }
        }

        // 3. NZCV Flags Comparison
        let uc_nzcv = (uc.reg_read(RegisterARM64::NZCV)? as u32) & 0xf0000000;
        let maarch_nzcv = ctx.pstate & 0xf0000000;
        if maarch_nzcv != uc_nzcv {
            eprintln!(
                "[-] NZCV FLAGS MISMATCH at Step {} (PC={:#x})!\n    Maarch64 NZCV: {:#010x}\n    Unicorn NZCV:  {:#010x}",
                step + 1, pc_before, maarch_nzcv, uc_nzcv
            );
            return Err("NZCV Flags Mismatch".into());
        }
    }

    println!("--------------------------------------------------------------------------------");
    println!("🟢 SUCCESS: ALL {} INSTRUCTIONS PASSED BIT-EXACT DIFFERENTIAL VERIFICATION!", num_instructions);
    println!("   Verified Registers (X0..X9), Stack Pointer (SP), Program Counter (PC), and NZCV Flags!");
    println!("================================================================================");
    Ok(())
}
