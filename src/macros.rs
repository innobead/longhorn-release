#[macro_export]
macro_rules! cmd {
    ($cmd:expr, $dir:expr, $args:expr $(, $success_handler:block)?) => {
        {
            let output: std::process::Output = std::process::Command::new($cmd).current_dir($dir).args($args).output()?;
            if output.status.success() {
                $($success_handler)?
            } else {
                use anyhow::anyhow;
                return Err(anyhow!("Command execution failed with: {:?} with {:?} {}, error_code={:?}", $cmd, $args, String::from_utf8(output.stderr.clone())?.trim(), output.status));
            }

            output
        }
    };
}

#[macro_export]
macro_rules! cmd_ignore_err {
    ($cmd:expr, $dir:expr, $args:expr $(, $success_handler:block)?) => {
        {
            let output: std::process::Output = std::process::Command::new($cmd).current_dir($dir).args($args).output()?;
            if output.status.success() {
                $($success_handler)?
            } else {
                use tracing_log::log;
                log::warn!("Command execution failed with: {:?} with {:?} \n{}", $cmd, $args, String::from_utf8(output.stderr.clone())?.trim());
            }

            output
        }
    }
}
