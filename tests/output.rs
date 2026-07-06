use remote_restart_plugin::output::{error, info, warn};

#[test]
fn info_uses_green() {
    let mut output_bytes = Vec::new();

    info(&mut output_bytes, format_args!("service started")).expect("write info");

    let text = String::from_utf8(output_bytes).expect("output utf8");
    assert_eq!(text, "\u{1b}[32m[INFO] service started\u{1b}[0m\n");
}

#[test]
fn warn_uses_yellow() {
    let mut output_bytes = Vec::new();

    warn(&mut output_bytes, format_args!("service pending")).expect("write warn");

    let text = String::from_utf8(output_bytes).expect("output utf8");
    assert_eq!(text, "\u{1b}[33m[WARN] service pending\u{1b}[0m\n");
}

#[test]
fn error_uses_red() {
    let mut output_bytes = Vec::new();

    error(&mut output_bytes, format_args!("service failed")).expect("write error");

    let text = String::from_utf8(output_bytes).expect("output utf8");
    assert_eq!(text, "\u{1b}[31m[ERROR] service failed\u{1b}[0m\n");
}
