//! Persistence for locally-added marketplace catalogs (#5311).
//!
//! Catalogs live in one sibling JSON file next to the plugin registry's
//! `state.json`, under the same Ghosty-owned `plugins/` root, and reuse the
//! registry's audited persistence machinery: private parent directory, atomic
//! temp-file publication, no-follow opens, and an `fd_lock` sidecar lock so a
//! concurrent TUI cannot interleave `add`/`remove` with a read.
//!
//! This is deliberately not a second database — it is the same store pattern
//! with a different schema, and it hard-fails closed on a malformed file
//! rather than rewriting it.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::types::{MarketplaceCatalog, MarketplaceCatalogId};
use crate::plugins::registry::{
    ensure_private_plugin_state_directory, harden_plugin_state_file, open_existing_regular_file,
    open_state_lock, path_entry_exists, save_state_with_hardener, state_lock_path,
    validate_existing_plugin_state_parent,
};

const MARKETPLACE_SCHEMA_VERSION: u32 = 1;
const MARKETPLACE_STATE_FILE: &str = "marketplaces.json";

/// One stored catalog: the parsed result plus where the document was read
/// from, so relative sources resolve against the catalog's own location at
/// install time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMarketplaceCatalog {
    pub added_at: String,
    /// Absolute path the catalog document was read from.
    pub source_path: String,
    pub catalog: MarketplaceCatalog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceState {
    schema_version: u32,
    #[serde(default)]
    catalogs: BTreeMap<String, StoredMarketplaceCatalog>,
}

impl Default for MarketplaceState {
    fn default() -> Self {
        Self {
            schema_version: MARKETPLACE_SCHEMA_VERSION,
            catalogs: BTreeMap::new(),
        }
    }
}

impl MarketplaceState {
    #[must_use]
    pub fn catalogs(&self) -> &BTreeMap<String, StoredMarketplaceCatalog> {
        &self.catalogs
    }

    pub fn get(&self, name: &str) -> Option<&StoredMarketplaceCatalog> {
        self.catalogs.get(name)
    }
}

/// Handle on the sibling catalog store. `None` from [`MarketplaceStore::open`]
/// means the registry has no persistence root at all (fail-closed registry);
/// callers report that honestly instead of inventing a location.
pub struct MarketplaceStore {
    path: PathBuf,
}

impl MarketplaceStore {
    /// Open the store that siblings the registry's `state.json`.
    #[must_use]
    pub fn open(state_path: Option<&Path>) -> Option<Self> {
        let parent = state_path?.parent()?.to_path_buf();
        Some(Self {
            path: parent.join(MARKETPLACE_STATE_FILE),
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read all catalogs. Never writes; never creates the file or its lock.
    pub fn load(&self) -> Result<MarketplaceState, String> {
        validate_existing_plugin_state_parent(&self.path)?;
        let lock_path = state_lock_path(&self.path);
        if path_entry_exists(&lock_path)? {
            let lock_file = open_state_lock(&lock_path, false)?;
            let lock = fd_lock::RwLock::new(lock_file);
            let _guard = lock
                .read()
                .map_err(|e| format!("failed to read-lock marketplace state: {e}"))?;
            return self.load_unlocked();
        }
        self.load_unlocked()
    }

    fn load_unlocked(&self) -> Result<MarketplaceState, String> {
        let Some(mut file) = open_existing_regular_file(&self.path, false)? else {
            return Ok(MarketplaceState::default());
        };
        let mut raw = String::new();
        file.read_to_string(&mut raw)
            .map_err(|e| format!("failed to read {}: {e}", self.path.display()))?;
        let state: MarketplaceState = serde_json::from_str(&raw)
            .map_err(|e| format!("failed to parse {}: {e}", self.path.display()))?;
        if state.schema_version != MARKETPLACE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported marketplace state schema {}; expected {MARKETPLACE_SCHEMA_VERSION} at {}",
                state.schema_version,
                self.path.display()
            ));
        }
        Ok(state)
    }

    /// Insert a catalog under `id`, refusing to replace an existing entry.
    pub fn add(
        &self,
        id: &MarketplaceCatalogId,
        entry: StoredMarketplaceCatalog,
    ) -> Result<(), String> {
        self.mutate(|state| {
            if state.catalogs.contains_key(id.as_str()) {
                return Err(format!(
                    "a marketplace named `{}` already exists; /plugin marketplace remove {} first",
                    id.as_str(),
                    id.as_str()
                ));
            }
            state.catalogs.insert(id.as_str().to_string(), entry);
            Ok(())
        })
    }

    /// Remove a catalog by name. `Ok(false)` means it was not stored.
    pub fn remove(&self, name: &str) -> Result<bool, String> {
        self.mutate(|state| Ok(state.catalogs.remove(name).is_some()))
    }

    fn mutate<R>(
        &self,
        mutate: impl FnOnce(&mut MarketplaceState) -> Result<R, String>,
    ) -> Result<R, String> {
        let lock_path = state_lock_path(&self.path);
        if let Some(parent) = lock_path.parent() {
            ensure_private_plugin_state_directory(parent)?;
        }
        let lock_file = open_state_lock(&lock_path, true)?;
        let mut lock = fd_lock::RwLock::new(lock_file);
        let _guard = lock
            .write()
            .map_err(|e| format!("failed to lock marketplace state for update: {e}"))?;
        let mut next = self.load_unlocked()?;
        let result = mutate(&mut next)?;
        save_state_with_hardener(&self.path, &next, harden_plugin_state_file)?;
        Ok(result)
    }
}
