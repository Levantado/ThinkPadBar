use std::{path::Path, process::Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegedWritePath {
    Direct,
    Pkexec,
}

pub async fn write_file_with_pkexec_fallback(
    path: &Path,
    contents: &str,
) -> Result<PrivilegedWritePath, String> {
    if let Ok(true) = write_file_direct(path, contents).await {
        return Ok(PrivilegedWritePath::Direct);
    }

    write_file_via_pkexec(path, contents).await?;
    Ok(PrivilegedWritePath::Pkexec)
}

pub async fn write_file_direct(path: &Path, contents: &str) -> Result<bool, String> {
    let path = path.to_path_buf();
    let contents = contents.to_string();

    tokio::task::spawn_blocking(move || std::fs::write(path, contents).map(|_| true))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

pub async fn write_file_via_pkexec(path: &Path, contents: &str) -> Result<(), String> {
    let script = format!(
        "printf '%s' {} > {}",
        shell_quote(contents),
        shell_quote(&path.display().to_string())
    );

    let output = tokio::process::Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(script)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|error| error.to_string())?;

    if output.status.success() {
        return Ok(());
    }

    Err(stderr_summary(&output.stderr)
        .unwrap_or_else(|| "pkexec command failed without stderr output".to_string()))
}

pub fn stderr_summary(stderr: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::{shell_quote, stderr_summary};

    #[test]
    fn shell_quote_escapes_single_quotes_for_sh() {
        assert_eq!(shell_quote("simple"), "'simple'");
        assert_eq!(shell_quote("fan '7'"), "'fan '\"'\"'7'\"'\"''");
    }

    #[test]
    fn stderr_summary_returns_first_non_empty_line() {
        assert_eq!(
            stderr_summary(b"\n\npkexec failed\nmore detail"),
            Some("pkexec failed".to_string())
        );
        assert_eq!(stderr_summary(b"\n \n"), None);
    }
}
