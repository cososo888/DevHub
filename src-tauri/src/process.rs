//! Bounded, fixed-argument process execution for detection probes.
//! The frontend can never provide a program, argument or environment value.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

fn read_limited(mut reader: impl Read + Send + 'static, limit: usize, over: Arc<AtomicBool>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    if bytes.len().saturating_add(n) > limit {
                        over.store(true, Ordering::SeqCst);
                        break;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                }
                Err(_) => break,
            }
        }
        bytes
    })
}

fn kill_group(child: &mut std::process::Child) {
    // The child was spawned as its own process group, so scripts (mvn/gradle) cannot
    // leave descendants behind after a timeout. Nothing here uses a shell.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
}

pub fn run(program: &Path, arguments: &[&str], timeout: Duration, limit: usize) -> Result<Output, String> {
    if !program.is_absolute() || !program.is_file() {
        return Err("检测程序不是固定的绝对文件".into());
    }
    let path = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin:/usr/sbin:/sbin".into());
    let home = std::env::var("HOME").unwrap_or_default();
    let mut command = Command::new(program);
    command
        .args(arguments)
        .env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("TERM", "dumb")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command.spawn().map_err(|error| format!("无法启动检测命令：{error}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "无法读取标准输出".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "无法读取标准错误".to_string())?;
    let over = Arc::new(AtomicBool::new(false));
    let out_reader = read_limited(stdout, limit, over.clone());
    let err_reader = read_limited(stderr, limit, over.clone());
    let started = Instant::now();
    let status;
    loop {
        if over.load(Ordering::SeqCst) {
            kill_group(&mut child);
            let _ = child.wait();
            let _ = out_reader.join();
            let _ = err_reader.join();
            return Err("检测命令输出超过安全上限".into());
        }
        match child.try_wait() {
            Ok(Some(value)) => { status = value; break; }
            Ok(None) => {
                if started.elapsed() > timeout {
                    kill_group(&mut child);
                    let _ = child.wait();
                    let _ = out_reader.join();
                    let _ = err_reader.join();
                    return Err("检测命令执行超时".into());
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                kill_group(&mut child);
                let _ = child.wait();
                let _ = out_reader.join();
                let _ = err_reader.join();
                return Err(format!("无法等待检测命令：{error}"));
            }
        }
    }
    let stdout = out_reader.join().map_err(|_| "读取标准输出失败".to_string())?;
    let stderr = err_reader.join().map_err(|_| "读取标准错误失败".to_string())?;
    if over.load(Ordering::SeqCst) {
        return Err("检测命令输出超过安全上限".into());
    }
    Ok(Output {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        status: status.code().unwrap_or(-1),
    })
}
