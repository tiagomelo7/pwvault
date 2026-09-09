use std::fs;
use std::path::{Path, PathBuf};
use gtk::glib;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn default_vault_path() -> PathBuf {
    let mut path = glib::user_data_dir();
    path.push("password-vault");
    path.push("vault.dat");

    path
}

#[cfg(unix)]
fn restrict_permissions(path: &Path, mode: u32) -> std::io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

pub fn read_vault_file(path: &Path) -> std::io::Result<Vec<u8>> {
    fs::read(path)
}

pub fn write_vault_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        restrict_permissions(parent, 0o700)?;
    }

    let temp_path = path.with_extension("tmp");

    fs::write(&temp_path, bytes)?;
    #[cfg(unix)]
    restrict_permissions(&temp_path, 0o600)?;
    fs::rename(&temp_path, path)?;
    #[cfg(unix)]
    restrict_permissions(path, 0o600)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn vault_file_permissions_are_restricted() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pwvault-perms-{unique}"));
        let file = dir.join("vault.dat");

        fs::create_dir_all(&dir).unwrap();
        write_vault_file(&file, b"abc").unwrap();

        #[cfg(unix)]
        {
            let file_mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
            let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;

            assert_eq!(file_mode, 0o600);
            assert_eq!(dir_mode, 0o700);
        }

        let _ = fs::remove_file(&file);
        let _ = fs::remove_dir_all(&dir);
    }
}
