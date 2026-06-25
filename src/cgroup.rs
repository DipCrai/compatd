/// Create cgroup subtree with delegation:
///   compatd cgroup --delegate docker /sys/fs/cgroup/system.slice/docker

use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Default)]
pub struct CgroupConfig {
    pub path: String,
    pub delegate: bool,
    pub user: Option<String>,
}

/// Create cgroup and set up delegation
pub fn setup(config: &CgroupConfig) {
    let cg_path = Path::new(&config.path);

    fs::create_dir_all(cg_path).unwrap_or_else(|e| {
        eprintln!("compatd: failed to create cgroup {}: {e}", config.path);
        std::process::exit(1);
    });

    if config.delegate {
        let controllers = fs::read_to_string(cg_path.join("cgroup.controllers"))
            .unwrap_or_default();
        let controllers = controllers.trim();

        if !controllers.is_empty() {
            let mut sc = fs::File::create(cg_path.join("cgroup.subtree_control"))
                .unwrap_or_else(|e| {
                    eprintln!("compatd: cannot set subtree_control: {e}");
                    std::process::exit(1);
                });

            for ctrl in controllers.split_whitespace() {
                let _ = write!(sc, "+{ctrl} ");
            }

            eprintln!("compatd: delegated controllers: {controllers}");
        }

        if let Some(user) = &config.user {
            set_cgroup_ownership(cg_path, user);
        }
    }

    println!("{}", config.path);
}

/// Set cgroup owner (chown -R)
fn set_cgroup_ownership(path: &Path, user: &str) {
    let uid = match user.parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            // Look up by name
            match uid_from_name(user) {
                Some(uid) => uid,
                None => {
                    eprintln!("compatd: unknown user '{user}'");
                    return;
                }
            }
        }
    };

    let path_str = path.to_string_lossy();
    let cpath = std::ffi::CString::new(path_str.as_ref()).unwrap();
    let ret = unsafe { libc::chown(cpath.as_ptr(), uid, uid) };
    if ret != 0 {
        eprintln!("compatd: chown {} failed: {}", path.display(), std::io::Error::last_os_error());
    }
}

fn uid_from_name(name: &str) -> Option<u32> {
    let cname = std::ffi::CString::new(name).ok()?;
    unsafe {
        let pw = libc::getpwnam(cname.as_ptr());
        if pw.is_null() {
            None
        } else {
            Some((*pw).pw_uid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgroup_config_defaults() {
        let cfg = CgroupConfig {
            path: "/sys/fs/cgroup/test".to_string(),
            delegate: true,
            user: None,
        };
        assert_eq!(cfg.path, "/sys/fs/cgroup/test");
        assert!(cfg.delegate);
    }
}
