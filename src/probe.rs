//! Modern-era MCP over newline-delimited JSON on child stdio.

use serde_json::{json, Value};
use std::ffi::OsStr;
use std::io::{self, BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

const FRAME_LIMIT: usize = 1024 * 1024;
const DISCOVER: &str = "server/discover";
const LIST: &str = "tools/list";

#[derive(Debug)]
pub struct ProbeResult {
    pub server_info: Value,
    pub tools: Value,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("no server executable supplied")]
    EmptyArgv,
    #[error("{0} timed out")]
    Timeout(&'static str),
    #[error("server closed stdout during {0}")]
    Eof(&'static str),
    #[error("{0} exceeded the response size limit")]
    FrameLimit(&'static str),
    #[error("invalid response to {0}")]
    Protocol(&'static str),
    #[error("server refused {0}: {1}")]
    Remote(&'static str, Value),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Spawn argv with optional environment overrides. Allow timeout per response,
/// including partial line reads. Kill and reap the child on return.
pub fn run(
    argv: &[impl AsRef<OsStr>],
    env: &[(&str, &str)],
    timeout: Duration,
) -> Result<ProbeResult, Error> {
    let (program, args) = argv.split_first().ok_or(Error::EmptyArgv)?;
    let mut child = Server(
        Command::new(program)
            .args(args)
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut input = child.0.stdin.take().expect("piped stdin");
    let mut output = BufReader::new(child.0.stdout.take().expect("piped stdout"));
    let discover = request(&mut input, &mut output, 1, DISCOVER, timeout)?;
    let server_info = &discover["_meta"]["io.modelcontextprotocol/serverInfo"];
    if !server_info["name"]
        .as_str()
        .is_some_and(|name| !name.is_empty())
        || !server_info["version"]
            .as_str()
            .is_some_and(|version| !version.is_empty())
    {
        return Err(Error::Protocol(DISCOVER));
    }
    let mut list = request(&mut input, &mut output, 2, LIST, timeout)?;
    let tools = list["tools"].take();
    if !tools.is_array() {
        return Err(Error::Protocol(LIST));
    }
    Ok(ProbeResult {
        server_info: server_info.clone(),
        tools,
    })
}

fn request(
    input: &mut ChildStdin,
    output: &mut BufReader<ChildStdout>,
    id: u64,
    method: &'static str,
    timeout: Duration,
) -> Result<Value, Error> {
    let mut request = serde_json::to_vec(&json!({
        "jsonrpc": "2.0", "id": id, "method": method,
        "params": {"_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": {}
        }}
    }))?;
    request.push(b'\n');
    input.write_all(&request)?;
    let started = Instant::now();
    let mut frame = Vec::new();
    loop {
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err(Error::Timeout(method));
        }
        if output.buffer().is_empty() {
            let mut poll = libc::pollfd {
                fd: output.get_ref().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let millis = remaining.as_millis().clamp(1, i32::MAX as u128) as i32;
            // The fd belongs to the live ChildStdout and poll receives one entry.
            match unsafe { libc::poll(&mut poll, 1, millis) } {
                -1 => {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err.into());
                }
                0 => continue,
                _ => {}
            }
        }
        let bytes = output.fill_buf()?;
        if bytes.is_empty() {
            return Err(Error::Eof(method));
        }
        let newline = bytes.iter().position(|byte| *byte == b'\n');
        let length = newline.map_or(bytes.len(), |index| index + 1);
        if frame.len() + length > FRAME_LIMIT {
            return Err(Error::FrameLimit(method));
        }
        frame.extend_from_slice(&bytes[..length]);
        output.consume(length);
        if newline.is_some() {
            break;
        }
    }
    let mut reply: Value = serde_json::from_slice(&frame)?;
    if reply["jsonrpc"] != "2.0" || reply["id"].as_u64() != Some(id) {
        return Err(Error::Protocol(method));
    }
    if let Some(error) = reply.get_mut("error") {
        return Err(Error::Remote(method, error.take()));
    }
    let result = reply["result"].take();
    if !result.is_object() {
        return Err(Error::Protocol(method));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immediate_exit_is_typed() {
        let result = run(&["sh", "-c", "exit 0"], &[], Duration::from_secs(1));
        assert!(
            matches!(result, Err(Error::Eof(DISCOVER)))
                || matches!(result, Err(Error::Io(ref err)) if err.kind() == io::ErrorKind::BrokenPipe)
        );
    }

    #[test]
    fn legacy_silence_and_partial_line_time_out() {
        for script in [
            "read -r line; exec sleep 5",
            "read -r line; printf '{'; exec sleep 5",
        ] {
            let result = run(&["sh", "-c", script], &[], Duration::from_millis(150));
            assert!(matches!(result, Err(Error::Timeout(DISCOVER))));
        }
    }
}
