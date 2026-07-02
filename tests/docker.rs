use remote_restart_plugin::command::{CommandOutput, CommandRunner};
use remote_restart_plugin::docker::{
    extract_host_port, first_network_name, start_container_with_sleep,
};
use remote_restart_plugin::error::AppResult;
use serde_json::json;
use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

#[test]
fn extracts_host_port_bound_to_container_8080() {
    let inspect_payload = json!([
        {
            "NetworkSettings": {
                "Ports": {
                    "8080/tcp": [
                        {"HostIp": "0.0.0.0", "HostPort": "9904"},
                        {"HostIp": "::", "HostPort": "9904"}
                    ],
                    "8009/tcp": [{"HostIp": "0.0.0.0", "HostPort": "9909"}]
                }
            }
        }
    ]);

    assert_eq!(
        extract_host_port(&inspect_payload, "8080/tcp"),
        Some("9904".to_string())
    );
}

#[test]
fn extracts_first_network_name_from_inspect_payload() {
    let inspect_payload = json!([
        {
            "NetworkSettings": {
                "Networks": {
                    "bridge": {},
                    "internal": {}
                }
            }
        }
    ]);

    assert_eq!(
        first_network_name(&inspect_payload),
        Some("bridge".to_string())
    );
}

#[test]
fn reconnects_network_when_docker_start_reports_attach_failure() {
    let runner = FakeRunner::new(vec![
        CommandOutput {
            code: 1,
            stdout: String::new(),
            stderr: "attaching to network failed".to_string(),
        },
        CommandOutput {
            code: 0,
            stdout: r#"[{"NetworkSettings":{"Networks":{"bridge":{}}}}]"#.to_string(),
            stderr: String::new(),
        },
        CommandOutput {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
        CommandOutput {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
        CommandOutput {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
    ]);
    let mut output = Vec::new();
    let sleeps = RefCell::new(Vec::new());

    let started = start_container_with_sleep(&runner, "frank", &mut output, |duration| {
        sleeps.borrow_mut().push(duration);
    })
    .expect("start should run");

    assert!(started);
    assert_eq!(
        runner.commands.borrow().as_slice(),
        &[
            "docker start frank",
            "docker inspect frank",
            "docker network disconnect bridge frank",
            "docker network connect bridge frank",
            "docker start frank",
        ]
    );
    assert_eq!(
        sleeps.borrow().as_slice(),
        &[Duration::from_secs(3), Duration::from_secs(1)]
    );
    let text = String::from_utf8(output).expect("output utf8");
    assert!(text.contains("Network connected successfully"));
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
