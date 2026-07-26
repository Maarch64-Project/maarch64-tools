use clap::Parser;
use maarch64_core::{cpu::CpuContext, interp::Interpreter, loader::ElfLoader, memory::MemoryManager};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "Maarch64 Binary Translator Runner", long_about = None)]
struct Args {
    /// Enable verbose execution logging to stderr
    #[arg(short, long)]
    verbose: bool,

    /// Path to target ARM64 ELF binary
    #[arg(value_name = "BINARY")]
    binary: PathBuf,

    /// Arguments to pass to target binary
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let filter = if args.verbose {
        EnvFilter::new("info")
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("off"))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("[+] Maarch64 Runner loading binary: {:?}", args.binary);
    let mut mem = MemoryManager::new();

    let bin_path = args.binary.to_str().unwrap_or("");
    let bin_name = args.binary.file_name().and_then(|f| f.to_str()).unwrap_or(bin_path);
    let mut target_args: Vec<&str> = Vec::new();
    target_args.push(bin_name);
    for a in &args.args {
        target_args.push(a.as_str());
    }

    let loaded = ElfLoader::load_file_with_args(&args.binary, &target_args, &mut mem)?;
    tracing::info!("[+] Loaded binary. Entry point: {:#x}", loaded.entry_point);

    let mut ctx = CpuContext::new();
    ctx.pc = loaded.entry_point;
    ctx.sp = loaded.stack_pointer;

    let mut thunk_manager = maarch64_thunks::ThunkManager::new();
    for (addr, name) in &loaded.dynamic_thunks {
        thunk_manager.resolve_dynamic_symbol(name, *addr);
    }
    tracing::info!("[+] Registered {} dynamic symbol thunks with ThunkManager", loaded.dynamic_thunks.len());

    tracing::info!("[+] Starting execution from PC = {:#x}", ctx.pc);
    let mut pc_history: std::collections::VecDeque<u64> = std::collections::VecDeque::with_capacity(30);
    let mut inst_count: u64 = 0;
    loop {
        inst_count += 1;
        if inst_count % 1_000_000 == 0 {
            tracing::info!("[Progress] {} instructions executed. PC = {:#x}", inst_count, ctx.pc);
        }
        if inst_count >= 50_000_000 {
            eprintln!("\n[!] Reached instruction limit (50,000,000). Current PC = {:#x}", ctx.pc);
            eprintln!("[!] Last 20 executed PCs:");
            for (i, pc) in pc_history.iter().enumerate() {
                eprintln!("    [{:2}] PC = {:#x}", i, pc);
            }
            break;
        }

        pc_history.push_back(ctx.pc);
        if pc_history.len() > 100 {
            pc_history.pop_front();
        }

        if let Some(thunk) = thunk_manager.get_thunk_by_address(ctx.pc) {
            let entry_pc = ctx.pc;
            let arg0 = ctx.get_x(0);
            let arg1 = ctx.get_x(1);
            let thunk_name = loaded.dynamic_thunks.iter().find(|(a, _)| *a == entry_pc).map(|(_, n)| n.as_str()).unwrap_or("unknown");
            if let Err(e) = thunk(&mut ctx, &mut mem) {
                eprintln!("[!] Thunk error: {}", e);
            }
            tracing::info!("[Thunk: {}] PC={:#x} (arg0={:#x}, arg1={:#x}) -> ret x0={:#x}", thunk_name, entry_pc, arg0, arg1, ctx.get_x(0));
            if ctx.pc == entry_pc {
                ctx.pc = ctx.get_x(30);
            }
            continue;
        }

        match Interpreter::step(&mut ctx, &mut mem) {
            Ok(true) => {},
            Ok(false) => break,
            Err(e) => {
                eprintln!("\n[!] Execution Error: {:?}", e);
                eprintln!("[!] Last 30 executed PCs:");
                for (i, pc) in pc_history.iter().enumerate() {
                    eprintln!("    [{:2}] PC = {:#x}", i, pc);
                }
                eprintln!("[!] CPU Registers on crash:");
                for i in 0..31 {
                    eprint!("X{:02}={:#018x} ", i, ctx.get_x(i as usize));
                    if (i + 1) % 4 == 0 { eprintln!(); }
                }
                eprintln!("\nSP={:#018x} PC={:#018x}", ctx.sp, ctx.pc);
                return Err(e.into());
            }
        }
    }

    tracing::info!("Execution Finished Cleanly");
    Ok(())
}
