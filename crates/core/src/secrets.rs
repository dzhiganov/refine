//! API key storage. macOS Keychain / Windows Credential Manager in the app; a plain file store
//! for Linux dev builds and tests.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::Result;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::error::Error;

pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>>;
    fn set(&self, account: &str, secret: &str) -> Result<()>;
    fn delete(&self, account: &str) -> Result<()>;
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub struct SystemStore {
    service: String,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl SystemStore {
    pub fn new(service: impl Into<String>) -> Self {
        SystemStore { service: service.into() }
    }

    fn entry(&self, account: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, account).map_err(|e| Error::Secret(e.to_string()))
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl SecretStore for SystemStore {
    fn get(&self, account: &str) -> Result<Option<String>> {
        match self.entry(account)?.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::Secret(e.to_string())),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<()> {
        self.entry(account)?.set_password(secret).map_err(|e| Error::Secret(e.to_string()))
    }

    fn delete(&self, account: &str) -> Result<()> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Secret(e.to_string())),
        }
    }
}

/// Keys in a JSON file readable only by the user. Not encrypted: dev/test use only.
pub struct FileStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl FileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        FileStore { path: path.into(), lock: Mutex::new(()) }
    }

    fn read(&self) -> Result<HashMap<String, String>> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => Ok(serde_json::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
            Err(e) => Err(e.into()),
        }
    }

    fn write(&self, map: &HashMap<String, String>) -> Result<()> {
        std::fs::write(&self.path, serde_json::to_vec_pretty(map)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

impl SecretStore for FileStore {
    fn get(&self, account: &str) -> Result<Option<String>> {
        let _g = self.lock.lock().unwrap();
        Ok(self.read()?.get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        let mut map = self.read()?;
        map.insert(account.into(), secret.into());
        self.write(&map)
    }

    fn delete(&self, account: &str) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        let mut map = self.read()?;
        if map.remove(account).is_some() {
            self.write(&map)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap().get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> Result<()> {
        self.0.lock().unwrap().insert(account.into(), secret.into());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<()> {
        self.0.lock().unwrap().remove(account);
        Ok(())
    }
}

/// The best store for this platform.
pub fn platform_store(data_dir: &std::path::Path) -> Box<dyn SecretStore> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        let _ = data_dir;
        Box::new(SystemStore::new("app.refine.desktop"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Box::new(FileStore::new(data_dir.join("secrets.json")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_store_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path().join("s.json"));
        assert_eq!(store.get("openrouter").unwrap(), None);
        store.set("openrouter", "sk-1").unwrap();
        assert_eq!(store.get("openrouter").unwrap().as_deref(), Some("sk-1"));
        store.delete("openrouter").unwrap();
        assert_eq!(store.get("openrouter").unwrap(), None);
    }
}
