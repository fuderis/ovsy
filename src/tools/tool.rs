use super::Tools;
use crate::{Manifest, prelude::*};
use std::{fs, process::Stdio, time::SystemTime};
use tokio::{fs as tfs, process::Command};

/// Time range to find recent file
const RECENT_FILE_TIME_RANGE: u64 = 120;

/// The tool structure
#[derive(Default, Clone, Debug)]
pub struct Tool {
    pub dir: PathBuf,
    pub manifest: Config<Manifest>,
    pub docs: Vec<String>,
    pub last_update: Option<SystemTime>,
    pub trace: Option<Trace>,
}

impl Tool {
    /// Reads a tool server & runs it
    pub(super) async fn run<P>(tool_dir: P) -> Result<Option<()>>
    where
        P: AsRef<Path>,
    {
        let tool_dir = tool_dir.as_ref();

        // read manifest:
        let manifest_path = tool_dir.join("Ovsy.toml");
        let manifest = match Config::<Manifest>::new(&manifest_path) {
            Ok(r) => r,
            Err(e) => {
                warn!("Read manifest '{}' error: {e}", manifest_path.display());
                return Ok(None);
            }
        };
        let tool_name = &manifest.tool.name;

        // check if tool already exists:
        if Tools::has(tool_name).await {
            trace!("Tool '{tool_name}' already running, skipping..");
            return Ok(None);
        }

        // remove tool if disabled:
        if !manifest.tool.enable {
            Tools::stop(tool_name).await?;
            return Ok(None);
        }

        // get actual mimetype:
        let exec_path = tool_dir.join(&manifest.tool.exec);
        let mut last_update: Option<SystemTime> = if manifest_path.exists() {
            Some(fs::metadata(&manifest_path)?.modified()?)
        } else {
            None
        };
        if exec_path.exists() {
            let exec_mtime = fs::metadata(&exec_path)?.modified()?;
            match last_update {
                Some(current) => last_update = Some(current.max(exec_mtime)),
                None => last_update = Some(exec_mtime),
            }
        }

        // generate prompt-doc:
        let mut docs = vec![];
        for (action_name, action) in &manifest.actions {
            let doc = fmt!(
                r#"* "{tool_name}/{action_name}":\n  * description: {descr}\n  * arguments: {args}\n  * examples: {exls}"#,
                descr = &action.description,
                args = {
                    let mut args = vec![];
                    for (name, arg) in &action.arguments {
                        args.push(fmt!(
                            r#"    * {}: format {}{}{}"#,
                            name,
                            arg.format,
                            if let Some(vars) = &arg.variants {
                                fmt!(", variants {:?}", vars)
                            } else {
                                String::new()
                            },
                            if arg.optional { ", optional" } else { "" },
                        ));
                    }
                    args.join("\n")
                },
                exls = {
                    let mut exls = vec![];
                    for (query, data) in &action.examples {
                        exls.push(fmt!(
                            r#"    * query: "{query}", tool call: ["{tool_name}/{action_name}", {data}]"#,
                            data = json::to_string(&data)?
                        ))
                    }
                    exls.join("\n")
                }
            );
            docs.push(doc);
        }

        // exec file path:
        let orig_exec = exec_path.clone();
        let ovsy_exec_name = fmt!(
            "ovsy-{}",
            manifest
                .tool
                .exec
                .file_name()
                .map(|s: &std::ffi::OsStr| str!(s.to_string_lossy()))
                .unwrap_or(tool_name.clone())
        );
        let ovsy_exec_path = tool_dir.join(&ovsy_exec_name);

        // check metadata:
        let needs_copy = if ovsy_exec_path.exists() {
            let ovsy_mtime = fs::metadata(&ovsy_exec_path)?.modified()?;
            if let Some(orig_mtime) = fs::metadata(&orig_exec)?.modified()?.into() {
                orig_mtime > ovsy_mtime
            } else {
                true
            }
        } else {
            true
        };

        // copy exec file (if needs):
        if needs_copy {
            if ovsy_exec_path.exists() {
                let _ = fs::remove_file(&ovsy_exec_path);
            }
            fs::copy(&orig_exec, &ovsy_exec_path)?;
        }

        // run tool server (if exists):
        if let Some(server) = &manifest.server {
            use tokio::net::TcpStream;

            // check port for available:
            let addr = fmt!("127.0.0.1:{}", server.port);
            if TcpStream::connect(&addr).await.is_err() {
                // create tool run command:
                let mut cmd = Command::new(ovsy_exec_path);
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());
                cmd.kill_on_drop(false);

                // spawn process child:
                let _ = cmd.spawn()?;
            }
        }

        sleep(Duration::from_millis(100)).await;

        // read new log file:
        let trace = {
            let logs_dir = app_data().join(&manifest.tool.name).join("logs");

            if let Some(log_file) =
                Self::find_recent_file(&logs_dir, RECENT_FILE_TIME_RANGE).await?
            {
                let timeout = Settings::get().tools.trace_timeout;
                Some(Trace::open(log_file, Duration::from_millis(timeout)).await?)
            } else {
                None
            }
        };

        // register tool instance:
        Tools::add(Tool {
            dir: tool_dir.to_path_buf(),
            manifest,
            last_update,
            docs,
            trace,
        })
        .await;

        Ok(Some(()))
    }

    /// Finds and returns the most recent file in dir
    async fn find_recent_file<P>(dir: &P, time_range: u64) -> Result<Option<PathBuf>>
    where
        P: AsRef<Path>,
    {
        let dir = dir.as_ref();
        // time range:
        let now = SystemTime::now();
        let time_start = now.checked_sub(Duration::from_secs(time_range)).unwrap();
        let time_end = now.checked_add(Duration::from_secs(time_range)).unwrap();

        let mut newest_file: Option<(PathBuf, SystemTime)> = None;
        let mut reader = tfs::read_dir(dir).await?;

        // read dir files:
        while let Some(entry) = reader.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let metadata = tfs::metadata(&path).await?;
                if let Ok(created) = metadata.created()
                    && created >= time_start
                    && created <= time_end
                {
                    // compare with last recent file:
                    if let Some((_, ref newest_time)) = newest_file {
                        if created > *newest_time {
                            newest_file = Some((path, created));
                        }
                    } else {
                        newest_file = Some((path, created));
                    }
                }
            }
        }

        Ok(newest_file.map(|(path, _)| path))
    }

    /// Checks & reruns tool if needs
    pub(super) async fn check(&self) -> Result<()> {
        let tool_dir = &self.dir;
        let manifest_path = tool_dir.join("Ovsy.toml");
        let name = &self.manifest.tool.name;

        // check manifest for exists:
        if !manifest_path.exists() {
            warn!(
                "Manifest '{}' not found, stopping tool '{name}'..",
                manifest_path.display()
            );
            Tools::stop(&self.manifest.tool.name).await?;
            return Ok(());
        }

        // check if still enabled:
        match Config::<Manifest>::new(&manifest_path) {
            Ok(new_manifest) => {
                if !new_manifest.tool.enable {
                    warn!("Tool '{name}' disabled in manifest, stopping..");
                    Tools::stop(name).await?;
                    return Ok(());
                }
            }
            Err(e) => {
                warn!(
                    "Fail with read manifest '{}': {e}..",
                    manifest_path.display()
                );
                Tools::stop(name).await?;
                return Ok(());
            }
        }

        // get actual mimetype:
        let exec_path = tool_dir.join(&self.manifest.tool.exec);
        let mut new_update: Option<SystemTime> = if manifest_path.exists() {
            Some(fs::metadata(&manifest_path)?.modified()?)
        } else {
            None
        };
        if exec_path.exists() {
            let exec_mtime = fs::metadata(&exec_path)?.modified()?;
            match new_update {
                Some(current) => new_update = Some(current.max(exec_mtime)),
                None => new_update = Some(exec_mtime),
            }
        }

        // rerun tool server (if outdated):
        match (self.last_update, new_update) {
            (Some(old), Some(new)) if new > old => {
                info!("Tool '{name}' outdated, restarting..");
                Tools::stop(name).await?;

                if let Some(()) = Self::run(tool_dir).await? {
                    info!("Tool '{name}' successfully restarted");
                }
            }
            _ => {
                // up to date
                trace!("Tool '{name}' is up to date");
            }
        }

        Ok(())
    }

    /// Stops the tool server
    pub(super) async fn stop(self) -> Result<()> {
        // get server port:
        if self.manifest.server.is_none() {
            return Ok(());
        }
        let server = self.manifest.server.as_ref().unwrap();
        let port = server.port;
        info!("Trying to kill server on {port} port..");

        // kill tool server by port:
        #[cfg(unix)]
        {
            // find server process by port:
            let output = Command::new("lsof")
                .args(["-t", "-i", &fmt!("TCP:{port}"), "-i", &fmt!("UDP:{port}")])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await?;

            if !output.status.success() {
                info!("Port {port} already is free");
                return Ok(());
            }

            // parse output PIDs:
            let pids: Vec<i32> = String::from_utf8_lossy(&output.stdout)
                .trim()
                .split('\n')
                .filter_map(|line| {
                    line.trim().parse().ok().filter(|&pid| pid > 1) // exclude system ones
                })
                .collect();

            if pids.is_empty() {
                info!("No processes found on port {port}");
                return Ok(());
            }

            info!("Found {} processes on port {port}: {pids:?}", pids.len());

            // stop all PIDs:
            for &pid in &pids {
                let pid_str = pid.to_string();
                if Command::new("kill")
                    .args(["-TERM", &pid_str])
                    .status()
                    .await?
                    .success()
                {
                    info!("Graceful stop PID {pid}");
                } else {
                    let _ = Command::new("kill").args(["-9", &pid_str]).status().await;
                    info!("Force kill PID {pid}");
                }
            }
        }

        /* TODO: kill server process on Windows OS..
        #[cfg(windows)]
        {
            // netstat -ano | findstr :PORT
            let output = Command::new("netstat")
                .args(["-ano", &format!("|findstr :{}", port)])
                .stdout(Stdio::piped())
                .output()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut pids = Vec::new();

            for line in stdout.lines() {
                if let Some(pid_str) = line.split_whitespace().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }

            for pid in pids {
                // taskkill /PID <pid> /F
                let status = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .status()?;

                if status.success() {
                    info!("Killed PID {} on port {}", pid, port);
                }
            }
        }
        */

        Ok(())
    }
}
