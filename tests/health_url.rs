use remote_restart_plugin::command::{CommandOutput, CommandRunner};
use remote_restart_plugin::config::AppConfig;
use remote_restart_plugin::error::AppResult;
use std::cell::RefCell;
use std::path::Path;

#[test]
fn resolve_health_url_prefers_explicit_health_url() {
    let runner = FakeRunner::new(vec![]);
    let config = AppConfig {
        health_url: Some("https://external.example.com/health".to_string()),
        ..AppConfig::default()
    };

    let url =
        remote_restart_plugin::resolve_health_url(&runner, "orders", &config).expect("resolve");

    assert_eq!(url, "https://external.example.com/health");
    assert!(runner.commands.borrow().is_empty());
}

#[test]
fn resolve_health_url_falls_back_to_inspect_when_override_missing() {
    let runner = FakeRunner::new(vec![CommandOutput {
        code: 0,
        stdout: r#"[{"NetworkSettings":{"Ports":{"8080/tcp":[{"HostPort":"9904"}]}}}]"#.to_string(),
        stderr: String::new(),
    }]);
    let config = AppConfig::default();

    let url =
        remote_restart_plugin::resolve_health_url(&runner, "orders", &config).expect("resolve");

    assert_eq!(url, "http://localhost:9904/actuator/health");
    assert_eq!(
        runner.commands.borrow().as_slice(),
        &["docker inspect orders"]
    );
}

struct FakeRunner {
    responses: RefCell<Vec<CommandOutput>>,
    commands: RefCell<Vec<String>>,
}

impl FakeRunner {
    fn new(mut responses: Vec<CommandOutput>) -> Self {
        responses.reverse();
        Self {
            responses: RefCell::new(responses),
            commands: RefCell::new(Vec::new()),
        }
    }

    fn next_response(&self, program: &str, args: &[String]) -> CommandOutput {
        self.commands
            .borrow_mut()
            .push(format!("{program} {}", args.join(" ")));
        self.responses
            .borrow_mut()
            .pop()
            .expect("missing fake response")
    }
}

impl CommandRunner for FakeRunner {
    fn run_capture(
        &self,
        program: &str,
        args: &[String],
        _cwd: Option<&Path>,
    ) -> AppResult<CommandOutput> {
        Ok(self.next_response(program, args))
    }

    fn run_inherit(
        &self,
        program: &str,
        args: &[String],
        _cwd: Option<&Path>,
    ) -> AppResult<CommandOutput> {
        Ok(self.next_response(program, args))
    }
}
