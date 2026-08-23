use clap::ValueEnum;

/// How the post-transfer chmod is issued on the target.
#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ChmodMethod {
    /// ponytail: perl builtin chmod — no execve("/bin/chmod") on the wire.
    /// Target has no perl? Use --chmod-method direct.
    Perl,
    /// Plain /bin/chmod — visible as its own execve, but always present.
    Direct,
}

impl ChmodMethod {
    pub fn cmd(&self, mode: &str, path: &str) -> String {
        let mode = mode.trim_start_matches('0');
        match self {
            ChmodMethod::Perl => format!("perl -e 'chmod 0{}, \"{}\"'", mode, path),
            ChmodMethod::Direct => format!("chmod 0{} \"{}\"", mode, path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chmod_cmds_apply_mode() {
        for m in [ChmodMethod::Perl, ChmodMethod::Direct] {
            let tmp = std::env::temp_dir().join("ordo_test_chmod");
            std::fs::write(&tmp, b"x").unwrap();
            let cmd = m.cmd("755", tmp.to_str().unwrap());
            let st = std::process::Command::new("sh").arg("-c").arg(&cmd).status().unwrap();
            assert!(st.success());
            let mode = std::fs::metadata(&tmp).unwrap().permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(mode.mode() & 0o777, 0o755);
            }
            let _ = std::fs::remove_file(&tmp);
        }
    }
}
