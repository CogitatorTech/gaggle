use crate::error::GaggleError;
use parking_lot::RwLock;
use std::fs;

static CREDENTIALS: once_cell::sync::Lazy<RwLock<Option<KaggleCredentials>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(None));

#[derive(Debug, Clone)]
pub struct KaggleCredentials {
    pub username: String,
    pub key: String,
}

/// Set Kaggle API credentials
pub fn set_credentials(username: &str, key: &str) -> Result<(), GaggleError> {
    let mut creds = CREDENTIALS.write();
    *creds = Some(KaggleCredentials {
        username: username.to_string(),
        key: key.to_string(),
    });
    Ok(())
}

/// Get stored credentials or try to load from environment/file
pub fn get_credentials() -> Result<KaggleCredentials, GaggleError> {
    // Check if credentials are already set in memory (fast path with read lock)
    if let Some(creds) = CREDENTIALS.read().as_ref() {
        return Ok(creds.clone());
    }

    // Acquire write lock to prevent race condition where multiple threads
    // try to load credentials simultaneously
    let mut creds_guard = CREDENTIALS.write();

    // Double-check after acquiring write lock (another thread may have loaded it)
    if let Some(creds) = creds_guard.as_ref() {
        return Ok(creds.clone());
    }

    // Try environment variables
    if let (Ok(username), Ok(key)) = (
        std::env::var("KAGGLE_USERNAME"),
        std::env::var("KAGGLE_KEY"),
    ) {
        let creds = KaggleCredentials { username, key };
        *creds_guard = Some(creds.clone());
        return Ok(creds);
    }

    // Try kaggle.json file
    let kaggle_json_path = dirs::home_dir()
        .ok_or_else(|| GaggleError::CredentialsError("Cannot find home directory".to_string()))?
        .join(".kaggle")
        .join("kaggle.json");

    if kaggle_json_path.exists() {
        // Verify file permissions for security (should not be world-readable)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&kaggle_json_path).map_err(|e| {
                GaggleError::CredentialsError(format!("Cannot read kaggle.json metadata: {}", e))
            })?;
            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                eprintln!(
                    "Warning: kaggle.json has overly permissive permissions. \
                     It should be readable only by the owner (chmod 600)."
                );
            }
        }

        let content = fs::read_to_string(&kaggle_json_path).map_err(|e| {
            GaggleError::CredentialsError(format!("Cannot read kaggle.json: {}", e))
        })?;

        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            GaggleError::CredentialsError(format!("Invalid JSON in kaggle.json: {}", e))
        })?;

        let username = json["username"]
            .as_str()
            .ok_or_else(|| {
                GaggleError::CredentialsError("Missing username in kaggle.json".to_string())
            })?
            .to_string();
        let key = json["key"]
            .as_str()
            .ok_or_else(|| GaggleError::CredentialsError("Missing key in kaggle.json".to_string()))?
            .to_string();

        let creds = KaggleCredentials { username, key };
        *creds_guard = Some(creds.clone());
        return Ok(creds);
    }

    Err(GaggleError::CredentialsError(
        "No Kaggle credentials found. Set KAGGLE_USERNAME and KAGGLE_KEY environment variables, \
         create ~/.kaggle/kaggle.json, or call gaggle_set_credentials()"
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_kaggle_credentials_struct() {
        let creds = KaggleCredentials {
            username: "testuser".to_string(),
            key: "testkey".to_string(),
        };
        assert_eq!(creds.username, "testuser");
        assert_eq!(creds.key, "testkey");
    }

    #[test]
    #[serial]
    fn test_set_credentials() {
        let result = set_credentials("user", "key");
        assert!(result.is_ok());

        let creds = CREDENTIALS.read();
        assert!(creds.is_some());
        let c = creds.as_ref().unwrap();
        assert_eq!(c.username, "user");
        assert_eq!(c.key, "key");
    }

    #[test]
    #[serial]
    fn test_get_credentials_after_set() {
        // Clear any existing credentials
        *CREDENTIALS.write() = None;

        set_credentials("test_user", "test_key").unwrap();
        let result = get_credentials();
        assert!(result.is_ok());
        let creds = result.unwrap();
        assert_eq!(creds.username, "test_user");
        assert_eq!(creds.key, "test_key");
    }

    #[test]
    #[serial]
    fn test_get_credentials_from_env() {
        // Clear in-memory credentials
        *CREDENTIALS.write() = None;

        std::env::set_var("KAGGLE_USERNAME", "env_user");
        std::env::set_var("KAGGLE_KEY", "env_key");

        let result = get_credentials();
        assert!(result.is_ok());
        let creds = result.unwrap();
        assert_eq!(creds.username, "env_user");
        assert_eq!(creds.key, "env_key");

        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");
    }

    #[test]
    #[serial]
    fn test_get_credentials_not_found() {
        // Clear everything
        *CREDENTIALS.write() = None;
        std::env::remove_var("KAGGLE_USERNAME");
        std::env::remove_var("KAGGLE_KEY");

        let result = get_credentials();
        // Note: This test might succeed if ~/.kaggle/kaggle.json exists
        // We're testing that without env vars or pre-set credentials,
        // it either finds kaggle.json or returns proper error
        match result {
            Ok(_) => {
                // kaggle.json file exists and was loaded successfully
            }
            Err(GaggleError::CredentialsError(msg)) => {
                assert!(msg.contains("No Kaggle credentials found"));
            }
            Err(_) => panic!("Expected CredentialsError or success"),
        }
    }

    #[test]
    #[serial]
    fn test_credentials_clone() {
        let creds1 = KaggleCredentials {
            username: "user".to_string(),
            key: "key".to_string(),
        };
        let creds2 = creds1.clone();
        assert_eq!(creds1.username, creds2.username);
        assert_eq!(creds1.key, creds2.key);
    }

    #[test]
    #[serial]
    fn test_credentials_debug() {
        let creds = KaggleCredentials {
            username: "user".to_string(),
            key: "key".to_string(),
        };
        let debug_str = format!("{:?}", creds);
        assert!(debug_str.contains("KaggleCredentials"));
    }

    #[test]
    #[serial]
    fn test_concurrent_credential_access() {
        *CREDENTIALS.write() = None;
        set_credentials("concurrent_user", "concurrent_key").unwrap();

        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let result = get_credentials();
                    assert!(result.is_ok());
                    let creds = result.unwrap();
                    assert_eq!(creds.username, "concurrent_user");
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    #[serial]
    fn test_concurrent_credential_updates() {
        *CREDENTIALS.write() = None;

        let barrier = Arc::new(std::sync::Barrier::new(5));
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let b = Arc::clone(&barrier);
                thread::spawn(move || {
                    b.wait();
                    set_credentials(&format!("user{}", i), &format!("key{}", i)).unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // After all updates, credentials should be set (to one of the values)
        let creds = get_credentials().unwrap();
        assert!(creds.username.starts_with("user"));
    }

    #[test]
    #[serial]
    fn test_env_credentials_partial() {
        *CREDENTIALS.write() = None;
        std::env::set_var("KAGGLE_USERNAME", "user_only");
        std::env::remove_var("KAGGLE_KEY");

        let result = get_credentials();
        // Should fail because key is missing in env vars
        // However, it might succeed if kaggle.json exists as fallback
        match result {
            Ok(creds) => {
                // kaggle.json provided credentials
                assert!(!creds.username.is_empty());
            }
            Err(_) => {
                // Expected: no kaggle.json, so partial env vars fail
            }
        }

        std::env::remove_var("KAGGLE_USERNAME");
    }

    #[test]
    #[serial]
    fn test_set_empty_credentials() {
        let result = set_credentials("", "");
        assert!(result.is_ok());

        let creds = get_credentials().unwrap();
        assert_eq!(creds.username, "");
        assert_eq!(creds.key, "");
    }
}
