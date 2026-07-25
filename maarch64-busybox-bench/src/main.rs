use maarch64_core::{cpu::CpuContext, interp::Interpreter, loader::ElfLoader, memory::MemoryManager};
use std::{collections::HashMap, path::PathBuf};

const APPLETS: &[&str] = &[
    "true", "false", "echo", "pwd", "uname", "whoami", "yes", "basename",
    "dirname", "expr", "test", "printf", "clear", "hostname",
    "ls", "cat", "mkdir", "rm", "cp", "mv", "stat", "date", "touch", "seq",
    "wc", "grep", "head", "tail", "sort", "uniq", "find", "sed", "awk",
    "diff", "tr", "tee", "xargs", "cal", "readlink", "realpath", "kill",
    "ps", "top", "df", "du", "uptime", "free", "id", "which", "chmod", "chown",
    "sync", "tar", "gzip", "gunzip", "zcat", "base64", "hexdump", "strings",
    "sh", "ash", "hush", "sleep", "usleep", "dmesg", "mknod", "mkfifo",
    "ln", "rmdir", "unlink", "readelf", "nm", "objdump", "install", "env",
];

#[derive(Debug)]
enum AppletStatus {
    Passed,
    Partial(String),
    Failed(String),
}

fn test_applet(busybox_path: &PathBuf, applet: &str) -> AppletStatus {
    let mut mem = MemoryManager::new();
    let loaded = match ElfLoader::load_file_with_args(busybox_path, &["busybox", applet], &mut mem) {
        Ok(l) => l,
        Err(e) => return AppletStatus::Failed(format!("Load error: {}", e)),
    };

    let mut ctx = CpuContext::new();
    ctx.pc = loaded.entry_point;
    ctx.sp = loaded.stack_pointer;

    let max_steps = 200_000;
    for _step_count in 0..max_steps {
        match Interpreter::step(&mut ctx, &mut mem) {
            Ok(true) => {}
            Ok(false) => {
                return AppletStatus::Passed;
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("Unimplemented") || err_str.contains("UnhandledSyscall") {
                    return AppletStatus::Partial(err_str);
                } else {
                    return AppletStatus::Failed(err_str);
                }
            }
        }
    }

    AppletStatus::Partial("Execution step limit reached (50,000 steps)".to_string())
}

fn main() {
    let busybox_path = PathBuf::from("tests/bin/busybox");
    if !busybox_path.exists() {
        eprintln!("[!] Error: tests/bin/busybox binary not found.");
        eprintln!("[!] Run `bash tests/build_fixtures.sh` or place busybox at tests/bin/busybox.");
        std::process::exit(1);
    }

    println!("================================================================================");
    println!("       Maarch64 BusyBox Applet Compatibility Benchmark Suite                    ");
    println!("================================================================================");
    println!("{:<14} {:<12} {:<50}", "APPLET", "STATUS", "DETAIL / STOPPING REASON");
    println!("--------------------------------------------------------------------------------");

    let mut passed_count = 0;
    let mut partial_count = 0;
    let mut failed_count = 0;
    let mut stopping_reasons: HashMap<String, usize> = HashMap::new();

    for &applet in APPLETS {
        let status = test_applet(&busybox_path, applet);
        match &status {
            AppletStatus::Passed => {
                passed_count += 1;
                println!("{:<14} \x1b[32mPASS\x1b[0m         Exited cleanly", applet);
            }
            AppletStatus::Partial(reason) => {
                partial_count += 1;
                *stopping_reasons.entry(reason.clone()).or_insert(0) += 1;
                let short_reason = if reason.len() > 48 { &reason[..48] } else { reason };
                println!("{:<14} \x1b[33mPARTIAL\x1b[0m      {}", applet, short_reason);
            }
            AppletStatus::Failed(reason) => {
                failed_count += 1;
                *stopping_reasons.entry(reason.clone()).or_insert(0) += 1;
                let short_reason = if reason.len() > 48 { &reason[..48] } else { reason };
                println!("{:<14} \x1b[31mFAIL\x1b[0m         {}", applet, short_reason);
            }
        }
    }

    let total = APPLETS.len();
    let pass_percent = (passed_count as f64 / total as f64) * 100.0;
    let partial_percent = (partial_count as f64 / total as f64) * 100.0;
    let fail_percent = (failed_count as f64 / total as f64) * 100.0;

    println!("--------------------------------------------------------------------------------");
    println!("SUMMARY STATISTICS:");
    println!("Total Applets Tested: {}", total);
    println!("--------------------------------------------------------------------------------");
    println!("  🟢 Passed (Exit Cleanly):            {:>3} ({:>5.1}%)", passed_count, pass_percent);
    println!("  🟡 Partial (Missing Opcode/Syscall): {:>3} ({:>5.1}%)", partial_count, partial_percent);
    println!("  🔴 Failed (Memory Fault/Error):      {:>3} ({:>5.1}%)", failed_count, fail_percent);
    println!("--------------------------------------------------------------------------------");
    println!("TOP BLOCKING OPCODES / SYSCALLS:");
    let mut sorted_reasons: Vec<_> = stopping_reasons.into_iter().collect();
    sorted_reasons.sort_by(|a, b| b.1.cmp(&a.1));
    for (i, (reason, count)) in sorted_reasons.iter().take(5).enumerate() {
        println!("  {}. {} ({} applets)", i + 1, reason, count);
    }
    println!("--------------------------------------------------------------------------------");
    println!("OVERALL BUSYBOX COMPATIBILITY SCORE: {:.1}%", pass_percent);
    println!("================================================================================");
}
