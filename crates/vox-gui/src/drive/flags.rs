#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DriveFlags {
    pub drive: bool,
    pub drive_headless: bool,
    pub show: bool,
}

pub fn parse_drive_flags(args: &[String]) -> DriveFlags {
    let mut flags = DriveFlags::default();
    for arg in args {
        match arg.as_str() {
            "--drive" => flags.drive = true,
            "--drive-headless" => flags.drive_headless = true,
            "--show" => flags.show = true,
            _ => {}
        }
    }
    if flags.drive_headless {
        flags.drive = false;
    }
    if std::env::var("VOX_GUI_DRIVE").ok().as_deref() == Some("1") && !flags.drive_headless {
        flags.drive = true;
    }
    if std::env::var("VOX_GUI_DRIVE_SHOW").ok().as_deref() == Some("1") {
        flags.show = true;
    }
    flags
}

pub fn read_token_from_file() -> Option<String> {
    use std::io::Read;
    let path = std::env::var("VOX_GUI_DRIVE_TOKEN_PATH").ok()?;
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW);
    }
    let mut f = opts.open(path).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_drive_and_show() {
        let f = parse_drive_flags(&["vox-gui".into(), "--drive".into(), "--show".into()]);
        assert!(f.drive);
        assert!(f.show);
        assert!(!f.drive_headless);
    }

    #[test]
    fn parse_headless_is_not_live() {
        let f = parse_drive_flags(&["vox-gui".into(), "--drive-headless".into()]);
        assert!(f.drive_headless);
        assert!(!f.drive);
    }
}
