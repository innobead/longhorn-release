use std::process::Command;

fn main() {
    let mut version = execute("git", &["describe", "--tags", "--dirty"]);

    if version.is_empty() {
        version = format!("v{}", env!("CARGO_PKG_VERSION").to_string())
    }

    println!("cargo:rustc-env=VERSION={}", version);
}

fn execute(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).unwrap()
            } else {
                String::new()
            }
        })
        .unwrap_or_default()
}
