use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use anyhow::{Context, Result};
use tokio::{process::Child, process::Command as TokioCommand, time::sleep};

static CONTROL_PORT_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

pub(crate) struct LaunchOptions<'a> {
    pub(crate) port: u16,
    pub(crate) scenario: &'a str,
    pub(crate) write_run_bundle: bool,
    pub(crate) capture_grid: bool,
    pub(crate) capture_frames_hz: f32,
    pub(crate) no_capture_events: bool,
    /// Optional pass-through for `cf-app --tick-rate-hz`. 0 = use cf-app default.
    pub(crate) tick_rate_hz: u32,
    /// M4A: ACC-A flags forwarded to cf-app's `--ui-scale` / `--high-contrast` /
    /// `--captions on|off` / `--reduced-*`. Defaults match cf-app defaults so a
    /// caller that never set them passes the unmodified surface through.
    pub(crate) ui_scale: f32,
    pub(crate) high_contrast: bool,
    pub(crate) captions: bool,
    pub(crate) reduced_motion: bool,
    pub(crate) reduced_shake: bool,
    pub(crate) reduced_flash: bool,
    /// races through sim.run_for_ticks budgets without per-tick wall-clock
    /// pacing.
    pub(crate) unpaced: bool,
}

pub(crate) struct LaunchedApp {
    pub(crate) child: Child,
    pub(crate) control_port_file: Option<PathBuf>,
}

pub(crate) fn launch_cf_app(opts: LaunchOptions<'_>) -> Result<LaunchedApp> {
    let bin = locate_cf_app_binary()?;
    let control_port_file = if opts.port == 0 {
        Some(unique_control_port_file())
    } else {
        None
    };
    let args = build_cf_app_args(&opts, control_port_file.as_deref());
    // Inherit stdio from the parent so cf-app's diagnostics (especially the
    // bevy_render screenshot INFO lines, ~10/sec under --capture-grid) flow
    // straight to the user's terminal. Piping with Stdio::piped() filled the
    // 64KB pipe buffer in seconds and deadlocked cf-app's render systems
    // when nobody was draining the pipe — the BP2 capture-grid freeze the
    // M2.5 win script kept hitting.
    let child = TokioCommand::new(&bin)
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin.display()))?;
    Ok(LaunchedApp {
        child,
        control_port_file,
    })
}

pub(crate) fn build_cf_app_args(opts: &LaunchOptions<'_>, control_port_file: Option<&Path>) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--scenario".into(),
        opts.scenario.into(),
        "--control-api".into(),
        "--control-port".into(),
        opts.port.to_string(),
        "--ticks".into(),
        "0".into(),
    ];
    if let Some(path) = control_port_file.as_ref() {
        args.push("--control-port-file".into());
        args.push(path.display().to_string());
    }
    if opts.tick_rate_hz != 0 {
        args.push("--tick-rate-hz".into());
        args.push(opts.tick_rate_hz.to_string());
    }
    if !opts.capture_grid {
        // Default: keep the legacy headless path the M0/M1/M1.5 cf-e2e scripts use.
        args.push("--headless-smoke".into());
    }
    if opts.capture_grid {
        args.push("--capture-grid".into());
        args.push("--capture-frames-hz".into());
        args.push(format!("{}", opts.capture_frames_hz));
        if opts.no_capture_events {
            args.push("--no-capture-events".into());
        }
    }
    if opts.write_run_bundle {
        args.push("--write-run-bundle".into());
        args.push("--run-bundle-dir".into());
        args.push(cf_replay::resolve_run_bundle_root(None).display().to_string());
    }
    // M4A ACC-A floor: forward accessibility flags so the spawned cf-app's
    // observe.settings + run_manifest.json + cf-ui HUD reflect the harness's
    // requested posture. cf-app defaults match cf-e2e defaults for ui_scale
    // (1.0), captions (on), high_contrast (false), and the three reduced-*
    // flags (false), so emitting only when non-default keeps the spawn line
    // tight for legacy tests.
    if (opts.ui_scale - 1.0).abs() > f32::EPSILON {
        args.push("--ui-scale".into());
        args.push(format!("{}", opts.ui_scale));
    }
    if opts.high_contrast {
        args.push("--high-contrast".into());
    }
    if !opts.captions {
        args.push("--captions".into());
        args.push("off".into());
    }
    if opts.reduced_motion {
        args.push("--reduced-motion".into());
    }
    if opts.reduced_shake {
        args.push("--reduced-shake".into());
    }
    if opts.reduced_flash {
        args.push("--reduced-flash".into());
    }
    // cf-e2e is the source of truth for scripted actions. Windowed capture
    // still opens a Bevy window, but it must not ingest ambient keyboard or
    // gamepad input from the developer machine and corrupt the scenario path.
    args.push("--disable-local-input".into());
    if opts.unpaced {
        args.push("--unpaced".into());
    }
    args
}

pub(crate) fn unique_control_port_file() -> PathBuf {
    let seq = CONTROL_PORT_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cf_e2e_control_port_{}_{}.txt", std::process::id(), seq))
}

pub(crate) async fn wait_for_control_port_file(path: &Path, timeout: Duration) -> Result<u16> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let port = text
                    .trim()
                    .parse::<u16>()
                    .with_context(|| format!("parse control port file {}", path.display()))?;
                if port == 0 {
                    anyhow::bail!("control port file {} reported port 0", path.display());
                }
                let _ = std::fs::remove_file(path);
                return Ok(port);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| format!("read control port file {}", path.display())),
        }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for {}", path.display());
        }
        sleep(Duration::from_millis(20)).await;
    }
}

fn locate_cf_app_binary() -> Result<PathBuf> {
    if let Ok(bin) = std::env::var("CF_APP_BIN") {
        if !bin.is_empty() {
            let p = PathBuf::from(bin);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    let exe = std::env::current_exe().context("current_exe")?;
    let dir = exe.parent().context("cf-e2e binary has no parent dir")?;
    let candidates: Vec<PathBuf> = vec![
        dir.join("cf-app"),
        dir.join("cf-app.exe"),
        dir.parent().unwrap_or(Path::new("")).join("cf-app"),
        dir.parent().unwrap_or(Path::new("")).join("cf-app.exe"),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    anyhow::bail!("could not locate cf-app binary; set CF_APP_BIN or build cf-app first")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_for_control_port_file_reads_bound_port() {
        let path = unique_control_port_file();
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, "41234\n").unwrap();

        let port = wait_for_control_port_file(&path, Duration::from_secs(1)).await.unwrap();

        assert_eq!(port, 41234);
        assert!(!path.exists());
    }

    #[test]
    fn cf_app_args_disable_local_input_for_scripted_runs() {
        let port_path = Path::new("/tmp/cf-e2e-port.txt");
        let args = build_cf_app_args(
            &LaunchOptions {
                port: 0,
                scenario: "m4a_micro_breach_readability",
                write_run_bundle: true,
                capture_grid: true,
                capture_frames_hz: 30.0,
                no_capture_events: false,
                tick_rate_hz: 120,
                ui_scale: 2.0,
                high_contrast: true,
                captions: true,
                reduced_motion: true,
                reduced_shake: true,
                reduced_flash: true,
                unpaced: false,
            },
            Some(port_path),
        );

        assert!(args.contains(&"--disable-local-input".to_string()));
        assert!(args.contains(&"--control-port-file".to_string()));
        assert!(!args.contains(&"--headless-smoke".to_string()));
    }

    #[test]
    fn cf_app_args_preserve_explicit_control_port() {
        let args = build_cf_app_args(
            &LaunchOptions {
                port: 17900,
                scenario: "m0_blank",
                write_run_bundle: false,
                capture_grid: false,
                capture_frames_hz: 10.0,
                no_capture_events: false,
                tick_rate_hz: 60,
                ui_scale: 1.0,
                high_contrast: false,
                captions: true,
                reduced_motion: false,
                reduced_shake: false,
                reduced_flash: false,
                unpaced: false,
            },
            None,
        );

        let port_arg = args
            .iter()
            .position(|arg| arg == "--control-port")
            .and_then(|idx| args.get(idx + 1))
            .expect("control port value");
        assert_eq!(port_arg, "17900");
        assert!(!args.contains(&"--control-port-file".to_string()));
    }
}
