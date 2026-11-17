use crate::Event;
use tao::event_loop::EventLoopProxy;

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use nothingx_desktop::protocol;
    use std::{
        io::{BufRead, BufReader, Write},
        path::PathBuf,
        process::{Child, ChildStdin, Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    pub struct Transport {
        child: Child,
        input: Option<ChildStdin>,
        reader: Option<thread::JoinHandle<()>>,
    }
    impl Transport {
        pub fn start(proxy: EventLoopProxy<Event>) -> Result<Self, String> {
            let executable = std::env::current_exe().map_err(|e| e.to_string())?;
            let bundled = executable
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("Resources/nothingx-bluetooth"));
            let path = bundled.filter(|p| p.exists()).unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build/nothingx-bluetooth")
            });
            let mut child = Command::new(path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| {
                    format!("Bluetooth helper could not start: {e}. Run scripts/build-macos.sh.")
                })?;
            let input = child.stdin.take();
            let stdout = child
                .stdout
                .take()
                .ok_or("Bluetooth helper has no output pipe")?;
            let reader = thread::spawn(move || {
                // Helper output is bounded at its source; malformed messages are ignored.
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else {
                        break;
                    };
                    let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                        continue;
                    };
                    let event = match v["type"].as_str() {
                        Some("connected") => Some(Event::Connected(
                            v["name"].as_str().unwrap_or("Nothing device").into(),
                            v["address"].as_str().unwrap_or("").into(),
                        )),
                        Some("data") => v["hex"]
                            .as_str()
                            .and_then(protocol::decode_hex)
                            .map(Event::Data),
                        Some("disconnected") => Some(Event::Disconnected),
                        Some("log") => Some(Event::Log(v["message"].as_str().unwrap_or("").into())),
                        _ => None,
                    };
                    if let Some(event) = event
                        && proxy.send_event(event).is_err()
                    {
                        break;
                    }
                }
                let _ = proxy.send_event(Event::TransportExited);
            });
            Ok(Self {
                child,
                input,
                reader: Some(reader),
            })
        }
        pub fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
            let input = self.input.as_mut().ok_or("Bluetooth helper stopped")?;
            writeln!(
                input,
                "{}",
                serde_json::json!({"type":"write","hex":protocol::hex(bytes)})
            )
            .map_err(|e| e.to_string())
        }
        pub fn refresh(&mut self) {
            if let Some(input) = self.input.as_mut() {
                let _ = writeln!(input, "{{\"type\":\"refresh\"}}");
            }
        }
        pub fn stop(&mut self) {
            if let Some(mut input) = self.input.take() {
                let _ = writeln!(input, "{{\"type\":\"quit\"}}");
            }
            let deadline = Instant::now() + Duration::from_secs(4);
            while Instant::now() < deadline {
                if self.child.try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
            }
            let _ = self.child.wait();
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
        }
    }
    impl Drop for Transport {
        fn drop(&mut self) {
            self.stop();
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::Transport;
#[cfg(windows)]
#[path = "transport_windows.rs"]
mod windows;
#[cfg(windows)]
pub use windows::Transport;
