#[macro_export]
macro_rules! cmd {
    ($cmd:expr, $dir:expr, $args:expr $(, $success_handler:block)?) => {
        let output: std::process::Output = std::process::Command::new($cmd).current_dir($dir).args($args).output()?;
        if output.status.success() {
            $($success_handler)?
        } else {
            use anyhow::anyhow;
            return Err(anyhow!("{}, error_code={:?}", String::from_utf8(output.stderr)?.trim(), output.status));
        }
    };
}

#[macro_export]
macro_rules! cmd_ignore_err {
    ($cmd:expr, $dir:expr, $args:expr $(, $success_handler:block)?) => {
        let output: std::process::Output = std::process::Command::new($cmd).current_dir($dir).args($args).output()?;
        if output.status.success() {
            $($success_handler)?
        } else {
            use tracing_log::log;
            log::debug!("{}", String::from_utf8(output.stderr)?.trim());
        }
    }
}

