//! Persistent storage for bookmarks.
//!
//! Stores bookmark data as JSON files in the app data directory.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::model::CharacterBookmarks;

/// Manages bookmark storage across multiple characters.
pub struct BookmarkStore {
    /// Directory for storing bookmark files
    data_dir: PathBuf,
    /// Cached bookmarks by character ID
    cache: HashMap<u64, CharacterBookmarks>,
}

impl BookmarkStore {
    /// Create a new bookmark store.
    ///
    /// # Arguments
    /// * `data_dir` - The app data directory (from Tauri's app_data_dir)
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            cache: HashMap::new(),
        }
    }

    /// Get the path to a character's bookmark file.
    fn bookmark_path(&self, character_id: u64) -> PathBuf {
        self.data_dir.join(format!("bookmarks_{}.json", character_id))
    }

    /// Load bookmarks for a character from disk.
    pub fn load(&mut self, character_id: u64, character_name: &str) -> io::Result<&CharacterBookmarks> {
        // Check cache first
        if self.cache.contains_key(&character_id) {
            return Ok(self.cache.get(&character_id).unwrap());
        }

        let path = self.bookmark_path(character_id);
        let bookmarks = if path.exists() {
            let content = fs::read_to_string(&path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| {
                CharacterBookmarks::new(character_id, character_name.to_string())
            })
        } else {
            CharacterBookmarks::new(character_id, character_name.to_string())
        };

        self.cache.insert(character_id, bookmarks);
        Ok(self.cache.get(&character_id).unwrap())
    }

    /// Get a mutable reference to a character's bookmarks.
    ///
    /// Loads from disk if not cached.
    pub fn get_mut(&mut self, character_id: u64, character_name: &str) -> io::Result<&mut CharacterBookmarks> {
        if !self.cache.contains_key(&character_id) {
            self.load(character_id, character_name)?;
        }
        Ok(self.cache.get_mut(&character_id).unwrap())
    }

    /// Get a reference to a character's bookmarks.
    pub fn get(&self, character_id: u64) -> Option<&CharacterBookmarks> {
        self.cache.get(&character_id)
    }

    /// Save a character's bookmarks to disk.
    pub fn save(&self, character_id: u64) -> io::Result<()> {
        let bookmarks = match self.cache.get(&character_id) {
            Some(b) => b,
            None => return Ok(()), // Nothing to save
        };

        // Ensure directory exists
        fs::create_dir_all(&self.data_dir)?;

        let path = self.bookmark_path(character_id);
        let content = serde_json::to_string_pretty(bookmarks)?;
        fs::write(&path, content)?;

        Ok(())
    }

    /// Save all cached bookmarks to disk.
    pub fn save_all(&self) -> io::Result<()> {
        for character_id in self.cache.keys() {
            self.save(*character_id)?;
        }
        Ok(())
    }

    /// Get all cached character IDs.
    pub fn cached_characters(&self) -> Vec<u64> {
        self.cache.keys().copied().collect()
    }

    /// Clear the cache (doesn't delete files).
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// List all bookmark files in the data directory.
    pub fn list_all_characters(&self) -> io::Result<Vec<u64>> {
        let mut ids = Vec::new();

        if !self.data_dir.exists() {
            return Ok(ids);
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();

            // Parse bookmarks_NNNN.json
            if let Some(id_str) = filename
                .strip_prefix("bookmarks_")
                .and_then(|s| s.strip_suffix(".json"))
            {
                if let Ok(id) = id_str.parse::<u64>() {
                    ids.push(id);
                }
            }
        }

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::model::{BookmarkType, Run};
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_store_save_and_load() {
        let dir = tempdir().unwrap();
        let mut store = BookmarkStore::new(dir.path().to_path_buf());

        // Load (creates new)
        {
            let bookmarks = store.get_mut(12345, "TestChar").unwrap();
            let run_id = bookmarks.start_run(
                PathBuf::from("game.txt"),
                None,
                Duration::from_secs(100),
                Some("Jita".to_string()),
            );
            bookmarks.run_mut(run_id).unwrap().add_bookmark(
                BookmarkType::Highlight,
                Duration::from_secs(150),
                Some("Important!".to_string()),
            );
        }

        // Save
        store.save(12345).unwrap();

        // Create new store and load
        let mut store2 = BookmarkStore::new(dir.path().to_path_buf());
        let bookmarks = store2.load(12345, "TestChar").unwrap();

        assert_eq!(bookmarks.runs.len(), 1);
        assert_eq!(bookmarks.runs[0].bookmarks.len(), 1);
        assert_eq!(
            bookmarks.runs[0].bookmarks[0].label,
            Some("Important!".to_string())
        );
    }

    #[test]
    fn test_list_all_characters() {
        let dir = tempdir().unwrap();
        let mut store = BookmarkStore::new(dir.path().to_path_buf());

        // Create bookmarks for multiple characters
        store.get_mut(111, "CharA").unwrap();
        store.get_mut(222, "CharB").unwrap();
        store.get_mut(333, "CharC").unwrap();

        store.save_all().unwrap();

        // List from fresh store
        let store2 = BookmarkStore::new(dir.path().to_path_buf());
        let mut ids = store2.list_all_characters().unwrap();
        ids.sort();

        assert_eq!(ids, vec![111, 222, 333]);
    }

    #[test]
    fn test_cache_behavior() {
        let dir = tempdir().unwrap();
        let mut store = BookmarkStore::new(dir.path().to_path_buf());

        // First load
        store.load(12345, "TestChar").unwrap();
        assert_eq!(store.cached_characters().len(), 1);

        // Second load should use cache
        store.load(12345, "TestChar").unwrap();
        assert_eq!(store.cached_characters().len(), 1);

        // Load different character
        store.load(67890, "OtherChar").unwrap();
        assert_eq!(store.cached_characters().len(), 2);

        // Clear cache
        store.clear_cache();
        assert!(store.cached_characters().is_empty());
    }
}
