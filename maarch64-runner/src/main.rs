use clap::Parser;
use maarch64_core::{cpu::CpuContext, interp::Interpreter, loader::ElfLoader, memory::MemoryManager};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Maarch64 Binary Translator Runner", long_about = None)]
struct Args {
    /// Path to target ARM64 ELF binary
    #[arg(value_name = "BINARY")]
    binary: PathBuf,

    /// Arguments to pass to target binary
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    println!("[+] Maarch64 Runner loading binary: {:?}", args.binary);
    let mut mem = MemoryManager::new();

    let loaded = ElfLoader::load_file(&args.binary, &mut mem)?;
    println!("[+] Loaded binary. Entry point: {:#x}", loaded.entry_point);

    let mut ctx = CpuContext::new();
    ctx.pc = loaded.entry_point;
    ctx.sp = loaded.stack_pointer;

    println!("[+] Starting execution from PC = {:#x}\n--- Binary Output ---", ctx.pc);
    Interpreter::run(&mut ctx, &mut mem)?;

    println!("\n--- Execution Finished Cleanly ---");
    Ok(())
}
