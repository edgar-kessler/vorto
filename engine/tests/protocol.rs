use std::os::windows::process::CommandExt;
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn malformed_and_missing_model_requests_do_not_kill_worker() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vorto-engine"))
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "not valid json").unwrap();
    writeln!(
        input,
        "{}",
        serde_json::json!({"op":"load", "root":"Z:/", "model":"whisper-base"})
    )
    .unwrap();
    writeln!(
        input,
        "{}",
        serde_json::json!({"op":"download", "root":".", "model":"../../not-a-model"})
    )
    .unwrap();
    drop(input);
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(20) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Worker did not exit after stdin closed");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let errors = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| {
            serde_json::from_str::<serde_json::Value>(line).is_ok_and(|v| v["event"] == "error")
        })
        .count();
    assert_eq!(
        errors, 3,
        "Every failed request must return a terminal error"
    );
}
