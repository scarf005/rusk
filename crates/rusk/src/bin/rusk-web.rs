use std::{
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use serde_json::{Value, json};

const BYTES_PER_KIB: usize = 1024;
const BYTES_PER_MIB: u64 = 1024 * 1024;
const MAX_RUN_SOURCE_KIB: usize = 64;
const JSON_BODY_SIZE_MULTIPLIER: usize = 2;
const MAX_RUN_SOURCE_BYTES: usize = MAX_RUN_SOURCE_KIB * BYTES_PER_KIB;
const MAX_BODY_BYTES: usize = MAX_RUN_SOURCE_BYTES * JSON_BODY_SIZE_MULTIPLIER;
const DEFAULT_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_DIST: &str = "/app/web/dist";
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_MAX_CONNECTIONS: usize = 64;
const DEFAULT_MAX_CONCURRENT_RUNS: usize = 2;
const CONNECTION_THREAD_STACK_KIB: usize = 512;
const CHILD_FILE_SIZE_MIB: u64 = 64;
const CHILD_OPEN_FILES: u64 = 64;
const CHILD_PROCESSES: u64 = 64;
const CONNECTION_THREAD_STACK_BYTES: usize = CONNECTION_THREAD_STACK_KIB * BYTES_PER_KIB;
const CHILD_FILE_SIZE_BYTES: u64 = CHILD_FILE_SIZE_MIB * BYTES_PER_MIB;

static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_RUNS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug)]
struct ServerConfig {
    request_timeout: Duration,
    run_timeout: Duration,
    max_concurrent_runs: usize,
}

#[derive(Debug)]
struct CounterPermit(&'static AtomicUsize);

impl Drop for CounterPermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Debug)]
struct CommandResult {
    status: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
    elapsed_ms: f64,
}

#[derive(Debug)]
struct RunResponse {
    result: CommandResult,
    ok: bool,
    stage: &'static str,
    compile_ms: f64,
    run_ms: f64,
    total_ms: f64,
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn main() -> std::io::Result<()> {
    let addr = env::var("RUSK_WEB_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let dist =
        PathBuf::from(env::var("RUSK_WEB_DIST").unwrap_or_else(|_| DEFAULT_DIST.to_string()));
    let config = ServerConfig {
        request_timeout: Duration::from_millis(read_u64_env(
            "RUSK_REQUEST_TIMEOUT_MS",
            DEFAULT_REQUEST_TIMEOUT_MS,
        )),
        run_timeout: Duration::from_millis(read_u64_env("RUSK_RUN_TIMEOUT_MS", DEFAULT_TIMEOUT_MS)),
        max_concurrent_runs: read_usize_env(
            "RUSK_MAX_CONCURRENT_RUNS",
            DEFAULT_MAX_CONCURRENT_RUNS,
        ),
    };
    let max_connections = read_usize_env("RUSK_MAX_CONNECTIONS", DEFAULT_MAX_CONNECTIONS);
    let listener = TcpListener::bind(&addr)?;
    eprintln!("rusk-web listening on http://{addr}");

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let Some(permit) = try_acquire(&ACTIVE_CONNECTIONS, max_connections) else {
            let _ = stream.write_all(&json_response(
                503,
                json!({ "error": "too many connections" }),
            ));
            continue;
        };
        let dist = dist.clone();
        let _ = thread::Builder::new()
            .name("rusk-web-conn".to_string())
            .stack_size(CONNECTION_THREAD_STACK_BYTES)
            .spawn(move || {
                let _permit = permit;
                handle_connection(stream, &dist, config);
            });
    }

    Ok(())
}

fn read_u64_env(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn read_usize_env(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn try_acquire(counter: &'static AtomicUsize, max: usize) -> Option<CounterPermit> {
    loop {
        let current = counter.load(Ordering::Acquire);
        if current >= max {
            return None;
        }
        if counter
            .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return Some(CounterPermit(counter));
        }
    }
}

fn handle_connection(mut stream: TcpStream, dist: &Path, config: ServerConfig) {
    let _ = stream.set_read_timeout(Some(config.request_timeout));
    let _ = stream.set_write_timeout(Some(config.request_timeout));
    let response = match read_request(&mut stream) {
        Ok(request) if request.method == "POST" && request.path == "/api/run" => {
            handle_run_request(&request.body, config)
        }
        Ok(request) if request.method == "GET" || request.method == "HEAD" => {
            serve_static(dist, &request.path, request.method == "HEAD")
        }
        Ok(_) => http_response(
            405,
            "application/json",
            br#"{"error":"method not allowed"}"#,
            false,
        ),
        Err(error) => http_response(
            400,
            "application/json",
            json!({ "error": error }).to_string().as_bytes(),
            false,
        ),
    };
    let _ = stream.write_all(&response);
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("empty request".to_string());
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_BODY_BYTES {
            return Err("request too large".to_string());
        }
        if let Some(index) = find_header_end(&buffer) {
            break index;
        }
    };

    let header = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "missing request line".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "missing method".to_string())?
        .to_string();
    let raw_path = parts.next().ok_or_else(|| "missing path".to_string())?;
    let path = raw_path.split('?').next().unwrap_or(raw_path).to_string();
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err("request body too large".to_string());
    }

    let body_start = header_end + 4;
    let mut body = buffer[body_start..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Ok(HttpRequest { method, path, body })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn handle_run_request(body: &[u8], config: ServerConfig) -> Vec<u8> {
    let Some(_permit) = try_acquire(&ACTIVE_RUNS, config.max_concurrent_runs) else {
        return json_response(503, json!({ "error": "too many active runs" }));
    };
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return json_response(400, json!({ "error": "invalid json" }));
    };
    let Some(rust) = value.get("rust").and_then(Value::as_str) else {
        return json_response(400, json!({ "error": "rust must be a string" }));
    };
    if rust.len() > MAX_RUN_SOURCE_BYTES {
        return json_response(
            413,
            json!({ "error": format!("rust source exceeds {MAX_RUN_SOURCE_KIB} KiB run limit") }),
        );
    }

    match compile_and_run_rust(rust, config.run_timeout) {
        Ok(response) => json_response(200, run_response_json(response)),
        Err(error) => json_response(500, json!({ "error": error.to_string() })),
    }
}

fn compile_and_run_rust(rust: &str, timeout: Duration) -> std::io::Result<RunResponse> {
    let dir = temp_run_dir();
    fs::create_dir_all(&dir)?;
    let started = Instant::now();
    let source_path = dir.join("main.rs");
    let binary_path = dir.join("main");
    fs::write(&source_path, rust)?;

    let compile = run_command(
        "rustc",
        &[
            "--edition=2024",
            source_path.to_str().unwrap(),
            "-o",
            binary_path.to_str().unwrap(),
        ],
        &dir,
        timeout,
    );
    if compile.status != Some(0) || compile.timed_out {
        let total_ms = elapsed_ms(started);
        let compile_ms = compile.elapsed_ms;
        let _ = fs::remove_dir_all(dir);
        return Ok(RunResponse {
            result: compile,
            ok: false,
            stage: "compile",
            compile_ms,
            run_ms: 0.0,
            total_ms,
        });
    }

    let run = run_command(binary_path.to_str().unwrap(), &[], &dir, timeout);
    let total_ms = elapsed_ms(started);
    let compile_ms = compile.elapsed_ms;
    let run_ms = run.elapsed_ms;
    let ok = run.status == Some(0) && !run.timed_out;
    let _ = fs::remove_dir_all(dir);
    Ok(RunResponse {
        result: run,
        ok,
        stage: "run",
        compile_ms,
        run_ms,
        total_ms,
    })
}

fn temp_run_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    env::temp_dir().join(format!("rusk-run-{}-{nanos}", std::process::id()))
}

fn run_command(command: &str, args: &[&str], cwd: &Path, timeout: Duration) -> CommandResult {
    let started = Instant::now();
    let mut command = Command::new(command);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_child_process(&mut command, timeout);
    let child = command.spawn();
    let Ok(mut child) = child else {
        return CommandResult {
            status: None,
            stdout: String::new(),
            stderr: child.unwrap_err().to_string(),
            timed_out: false,
            elapsed_ms: elapsed_ms(started),
        };
    };
    let child_pid = child.id();

    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                kill_process_tree(child_pid);
                let _ = child.kill();
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                kill_process_tree(child_pid);
                return CommandResult {
                    status: None,
                    stdout: String::new(),
                    stderr: error.to_string(),
                    timed_out,
                    elapsed_ms: elapsed_ms(started),
                };
            }
        }
    }

    kill_process_tree(child_pid);
    match child.wait_with_output() {
        Ok(output) => CommandResult {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            timed_out,
            elapsed_ms: elapsed_ms(started),
        },
        Err(error) => CommandResult {
            status: None,
            stdout: String::new(),
            stderr: error.to_string(),
            timed_out,
            elapsed_ms: elapsed_ms(started),
        },
    }
}

#[cfg(unix)]
fn configure_child_process(command: &mut Command, timeout: Duration) {
    let cpu_seconds = timeout.as_secs().saturating_add(1).max(1);
    unsafe {
        command.pre_exec(move || {
            create_child_process_group()?;
            apply_child_limits(cpu_seconds)
        });
    }
}

#[cfg(not(unix))]
fn configure_child_process(_command: &mut Command, _timeout: Duration) {}

#[cfg(unix)]
fn create_child_process_group() -> std::io::Result<()> {
    if unsafe { libc::setpgid(0, 0) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn apply_child_limits(cpu_seconds: u64) -> std::io::Result<()> {
    set_limit(libc::RLIMIT_CPU, cpu_seconds, cpu_seconds)?;
    set_limit(libc::RLIMIT_CORE, 0, 0)?;
    set_limit(
        libc::RLIMIT_FSIZE,
        CHILD_FILE_SIZE_BYTES,
        CHILD_FILE_SIZE_BYTES,
    )?;
    set_limit(libc::RLIMIT_NOFILE, CHILD_OPEN_FILES, CHILD_OPEN_FILES)?;
    set_limit(libc::RLIMIT_NPROC, CHILD_PROCESSES, CHILD_PROCESSES)
}

#[cfg(unix)]
fn set_limit(resource: libc::__rlimit_resource_t, soft: u64, hard: u64) -> std::io::Result<()> {
    let limit = libc::rlimit {
        rlim_cur: soft as libc::rlim_t,
        rlim_max: hard as libc::rlim_t,
    };
    if unsafe { libc::setrlimit(resource, &limit) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn kill_process_tree(pid: u32) {
    let pgid = -(pid as i32);
    let _ = unsafe { libc::kill(pgid, libc::SIGKILL) };
}

#[cfg(not(unix))]
fn kill_process_tree(_pid: u32) {}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn run_response_json(response: RunResponse) -> Value {
    json!({
        "ok": response.ok,
        "stage": response.stage,
        "status": response.result.status,
        "stdout": response.result.stdout,
        "stderr": response.result.stderr,
        "timedOut": response.result.timed_out,
        "compileMs": response.compile_ms,
        "runMs": response.run_ms,
        "totalMs": response.total_ms,
    })
}

fn serve_static(dist: &Path, path: &str, head_only: bool) -> Vec<u8> {
    let relative = route_path(path);
    let file = safe_join(dist, &relative)
        .filter(|path| path.is_file())
        .or_else(|| {
            if is_app_route(path) {
                Some(dist.join("index.html"))
            } else {
                None
            }
        });
    let Some(file) = file else {
        return http_response(404, "text/plain; charset=utf-8", b"not found", head_only);
    };
    match fs::read(&file) {
        Ok(body) => http_response(200, content_type(&file), &body, head_only),
        Err(error) => json_response(500, json!({ "error": error.to_string() })),
    }
}

fn route_path(path: &str) -> String {
    if path == "/" {
        "index.html".to_string()
    } else {
        path.trim_start_matches('/').to_string()
    }
}

fn is_app_route(path: &str) -> bool {
    matches!(path, "/")
        || path.starts_with("/examples/")
        || path.starts_with("/ruk-examples/")
        || path.starts_with("/rust-examples/")
}

fn safe_join(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => path.push(part),
            _ => return None,
        }
    }
    Some(path)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

fn json_response(status: u16, value: Value) -> Vec<u8> {
    http_response(
        status,
        "application/json",
        value.to_string().as_bytes(),
        false,
    )
}

fn http_response(status: u16, content_type: &str, body: &[u8], head_only: bool) -> Vec<u8> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        503 => "Service Unavailable",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let headers = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len(),
    );
    if head_only {
        headers.into_bytes()
    } else {
        let mut response = headers.into_bytes();
        response.extend_from_slice(body);
        response
    }
}
