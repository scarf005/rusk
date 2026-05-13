use std::{
    env, fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    http::{
        HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri,
        header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};

const BYTES_PER_KIB: usize = 1024;
const BYTES_PER_MIB: u64 = 1024 * 1024;
const MAX_RUN_SOURCE_KIB: usize = 64;
const MAX_OUTPUT_KIB: usize = 256;
const JSON_BODY_SIZE_MULTIPLIER: usize = 2;
const MAX_RUN_SOURCE_BYTES: usize = MAX_RUN_SOURCE_KIB * BYTES_PER_KIB;
const MAX_OUTPUT_BYTES: usize = MAX_OUTPUT_KIB * BYTES_PER_KIB;
const MAX_BODY_BYTES: usize = MAX_RUN_SOURCE_BYTES * JSON_BODY_SIZE_MULTIPLIER;
const DEFAULT_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_DIST: &str = "/app/web/dist";
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_MAX_REQUESTS: usize = 64;
const DEFAULT_MAX_CONCURRENT_RUNS: usize = 2;
const DEFAULT_RUN_RATE_LIMIT_PER_MINUTE: usize = 10;
const RATE_LIMIT_WINDOW_MS: u64 = 60_000;
const CHILD_FILE_SIZE_MIB: u64 = 64;
const CHILD_OPEN_FILES: u64 = 64;
const CHILD_PROCESSES: u64 = 64;
const CHILD_FILE_SIZE_BYTES: u64 = CHILD_FILE_SIZE_MIB * BYTES_PER_MIB;

static ACTIVE_REQUESTS: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_RUNS: AtomicUsize = AtomicUsize::new(0);
static RUN_RATE_WINDOW_MS: AtomicU64 = AtomicU64::new(0);
static RUN_RATE_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
struct AppState {
    config: ServerConfig,
    dist: PathBuf,
}

#[derive(Clone, Copy, Debug)]
struct ServerConfig {
    request_timeout: Duration,
    run_timeout: Duration,
    max_requests: usize,
    max_concurrent_runs: usize,
    run_rate_limit_per_minute: usize,
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
    stdout_truncated: bool,
    stderr_truncated: bool,
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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = env::var("RUSK_WEB_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let state = AppState {
        config: ServerConfig {
            request_timeout: Duration::from_millis(read_u64_env(
                "RUSK_REQUEST_TIMEOUT_MS",
                DEFAULT_REQUEST_TIMEOUT_MS,
            )),
            run_timeout: Duration::from_millis(read_u64_env(
                "RUSK_RUN_TIMEOUT_MS",
                DEFAULT_TIMEOUT_MS,
            )),
            max_requests: read_usize_env("RUSK_MAX_REQUESTS", DEFAULT_MAX_REQUESTS),
            max_concurrent_runs: read_usize_env(
                "RUSK_MAX_CONCURRENT_RUNS",
                DEFAULT_MAX_CONCURRENT_RUNS,
            ),
            run_rate_limit_per_minute: read_usize_env(
                "RUSK_RUN_RATE_LIMIT_PER_MINUTE",
                DEFAULT_RUN_RATE_LIMIT_PER_MINUTE,
            ),
        },
        dist: PathBuf::from(env::var("RUSK_WEB_DIST").unwrap_or_else(|_| DEFAULT_DIST.to_string())),
    };
    let app = Router::new()
        .route("/api/run", post(handle_run_request))
        .fallback(handle_static_request)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            limit_active_requests,
        ))
        .layer(middleware::map_response(add_security_headers))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("rusk-web listening on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(std::io::Error::other)
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
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

async fn limit_active_requests(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let Some(_permit) = try_acquire(&ACTIVE_REQUESTS, state.config.max_requests) else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "error": "too many requests" }),
        );
    };
    tokio::time::timeout(state.config.request_timeout, next.run(request))
        .await
        .unwrap_or_else(|_| {
            json_response(
                StatusCode::REQUEST_TIMEOUT,
                json!({ "error": "request timed out" }),
            )
        })
}

async fn handle_run_request(State(state): State<AppState>, body: Bytes) -> Response {
    if !try_run_rate_limit(state.config.run_rate_limit_per_minute) {
        return json_response(
            StatusCode::TOO_MANY_REQUESTS,
            json!({ "error": "run rate limit exceeded" }),
        );
    }
    let Some(_permit) = try_acquire(&ACTIVE_RUNS, state.config.max_concurrent_runs) else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "error": "too many active runs" }),
        );
    };
    let Ok(value) = serde_json::from_slice::<Value>(&body) else {
        return json_response(StatusCode::BAD_REQUEST, json!({ "error": "invalid json" }));
    };
    let Some(rust) = value.get("rust").and_then(Value::as_str) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({ "error": "rust must be a string" }),
        );
    };
    if rust.len() > MAX_RUN_SOURCE_BYTES {
        return json_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            json!({ "error": format!("rust source exceeds {MAX_RUN_SOURCE_KIB} KiB run limit") }),
        );
    }

    let rust = rust.to_string();
    let timeout = state.config.run_timeout;
    match tokio::task::spawn_blocking(move || compile_and_run_rust(&rust, timeout)).await {
        Ok(Ok(response)) => json_response(StatusCode::OK, run_response_json(response)),
        Ok(Err(error)) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": error.to_string() }),
        ),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": error.to_string() }),
        ),
    }
}

fn try_run_rate_limit(max: usize) -> bool {
    let now = now_millis();
    loop {
        let window_started = RUN_RATE_WINDOW_MS.load(Ordering::Acquire);
        if now.saturating_sub(window_started) < RATE_LIMIT_WINDOW_MS {
            break;
        }
        if RUN_RATE_WINDOW_MS
            .compare_exchange(window_started, now, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            RUN_RATE_COUNT.store(0, Ordering::Release);
            break;
        }
    }
    let previous = RUN_RATE_COUNT.fetch_add(1, Ordering::AcqRel);
    if previous < max {
        true
    } else {
        RUN_RATE_COUNT.fetch_sub(1, Ordering::AcqRel);
        false
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
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
    let stdout_path = temp_output_path(cwd, "stdout");
    let stderr_path = temp_output_path(cwd, "stderr");
    let stdout_file = fs::File::create(&stdout_path);
    let stderr_file = fs::File::create(&stderr_path);
    let (Ok(stdout_file), Ok(stderr_file)) = (stdout_file, stderr_file) else {
        return CommandResult {
            status: None,
            stdout: String::new(),
            stderr: "failed to create command output files".to_string(),
            timed_out: false,
            stdout_truncated: false,
            stderr_truncated: false,
            elapsed_ms: elapsed_ms(started),
        };
    };

    let mut command = Command::new(command);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    configure_child_process(&mut command, timeout);
    let child = command.spawn();
    let Ok(mut child) = child else {
        let _ = fs::remove_file(stdout_path);
        let _ = fs::remove_file(stderr_path);
        return CommandResult {
            status: None,
            stdout: String::new(),
            stderr: child.unwrap_err().to_string(),
            timed_out: false,
            stdout_truncated: false,
            stderr_truncated: false,
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
                let _ = fs::remove_file(stdout_path);
                let _ = fs::remove_file(stderr_path);
                return CommandResult {
                    status: None,
                    stdout: String::new(),
                    stderr: error.to_string(),
                    timed_out,
                    stdout_truncated: false,
                    stderr_truncated: false,
                    elapsed_ms: elapsed_ms(started),
                };
            }
        }
    }

    kill_process_tree(child_pid);
    let status = child.wait().map(|status| status.code());
    let (stdout, stdout_truncated) = read_limited_utf8(&stdout_path, MAX_OUTPUT_BYTES);
    let (mut stderr, stderr_truncated) = read_limited_utf8(&stderr_path, MAX_OUTPUT_BYTES);
    let _ = fs::remove_file(stdout_path);
    let _ = fs::remove_file(stderr_path);
    match status {
        Ok(status) => CommandResult {
            status,
            stdout,
            stderr,
            timed_out,
            stdout_truncated,
            stderr_truncated,
            elapsed_ms: elapsed_ms(started),
        },
        Err(error) => {
            if !stderr.is_empty() {
                stderr.push('\n');
            }
            stderr.push_str(&error.to_string());
            CommandResult {
                status: None,
                stdout,
                stderr,
                timed_out,
                stdout_truncated,
                stderr_truncated,
                elapsed_ms: elapsed_ms(started),
            }
        }
    }
}

fn temp_output_path(cwd: &Path, name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    cwd.join(format!("{name}-{}-{nanos}.txt", std::process::id()))
}

fn read_limited_utf8(path: &Path, limit: usize) -> (String, bool) {
    let Ok(mut file) = fs::File::open(path) else {
        return (String::new(), false);
    };
    let mut buffer = vec![0; limit.saturating_add(1)];
    let bytes_read = file.read(&mut buffer).unwrap_or_default();
    let truncated = bytes_read > limit;
    buffer.truncate(bytes_read.min(limit));
    (String::from_utf8_lossy(&buffer).to_string(), truncated)
}

#[cfg(unix)]
fn configure_child_process(command: &mut Command, timeout: Duration) {
    let cpu_seconds = timeout.as_secs().saturating_add(1).max(1);
    unsafe {
        command.pre_exec(move || {
            create_child_process_group()?;
            apply_child_sandbox();
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
fn apply_child_sandbox() {
    let _ = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    let _ = unsafe { libc::unshare(libc::CLONE_NEWNET) };
    let _ = unsafe { libc::unshare(libc::CLONE_NEWIPC) };
    let _ = unsafe { libc::unshare(libc::CLONE_NEWUTS) };
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
        "stdoutTruncated": response.result.stdout_truncated,
        "stderrTruncated": response.result.stderr_truncated,
        "compileMs": response.compile_ms,
        "runMs": response.run_ms,
        "totalMs": response.total_ms,
    })
}

async fn handle_static_request(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return json_response(
            StatusCode::METHOD_NOT_ALLOWED,
            json!({ "error": "method not allowed" }),
        );
    }
    serve_static(&state.dist, uri.path(), method == Method::HEAD)
}

fn serve_static(dist: &Path, path: &str, head_only: bool) -> Response {
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
        return text_response(StatusCode::NOT_FOUND, "not found", head_only);
    };
    match fs::read(&file) {
        Ok(body) => body_response(StatusCode::OK, content_type(&file), body, head_only),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": error.to_string() }),
        ),
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

fn json_response(status: StatusCode, value: Value) -> Response {
    (status, Json(value)).into_response()
}

fn text_response(status: StatusCode, body: &'static str, head_only: bool) -> Response {
    body_response(
        status,
        "text/plain; charset=utf-8",
        body.as_bytes().to_vec(),
        head_only,
    )
}

fn body_response(
    status: StatusCode,
    content_type: &'static str,
    body: Vec<u8>,
    head_only: bool,
) -> Response {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .header(CONTENT_LENGTH, body.len().to_string())
        .body(if head_only {
            Body::empty()
        } else {
            Body::from(body)
        })
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn add_security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    insert_static_header(
        headers,
        "content-security-policy",
        "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; form-action 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'",
    );
    insert_static_header(headers, "cross-origin-embedder-policy", "require-corp");
    insert_static_header(headers, "cross-origin-opener-policy", "same-origin");
    insert_static_header(headers, "cross-origin-resource-policy", "same-origin");
    insert_static_header(
        headers,
        "permissions-policy",
        "camera=(), microphone=(), geolocation=(), payment=(), usb=(), serial=(), bluetooth=(), interest-cohort=()",
    );
    insert_static_header(headers, "referrer-policy", "no-referrer");
    insert_static_header(
        headers,
        "strict-transport-security",
        "max-age=31536000; includeSubDomains; preload",
    );
    insert_static_header(headers, "x-content-type-options", "nosniff");
    insert_static_header(headers, "x-frame-options", "DENY");
    response
}

fn insert_static_header(headers: &mut HeaderMap, name: &'static str, value: &'static str) {
    headers.insert(
        HeaderName::from_static(name),
        HeaderValue::from_static(value),
    );
}
