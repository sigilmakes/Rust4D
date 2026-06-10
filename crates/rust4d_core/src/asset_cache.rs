//! Asset caching and hot-reload system
//!
//! This module provides a type-erased asset cache that can store any type
//! implementing the [`Asset`] trait. Assets are loaded from files, cached
//! with reference counting, and can be hot-reloaded when files change on disk.
//!
//! # Architecture
//!
//! - [`AssetId`] - Unique identifier for a cached asset (incrementing `u64`)
//! - [`AssetHandle`] - Lightweight handle returned to callers, containing id and path
//! - [`Asset`] trait - Implemented by types that can be loaded from files
//! - [`AssetCache`] - Main cache storing `Arc<dyn Any + Send + Sync>` internally
//!
//! # Example
//!
//! ```ignore
//! let mut cache = AssetCache::new();
//! let handle = cache.load::<MyAsset>("assets/model.ron")?;
//! let data: Arc<MyAsset> = cache.get::<MyAsset>(&handle).unwrap();
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

// Import from sibling module. When wired into lib.rs this becomes `crate::asset_error::AssetError`.
// For now, we use a path that will work once lib.rs declares both modules.
use super::asset_error::AssetError;

/// Unique identifier for an asset in the cache.
///
/// Asset IDs are assigned sequentially starting from 1. An ID of 0 is reserved
/// and never assigned to a valid asset.
pub type AssetId = u64;

/// A lightweight handle to a cached asset.
///
/// Handles are returned by [`AssetCache::load`] and can be used to retrieve
/// the asset data via [`AssetCache::get`]. They are cheap to clone and can
/// be stored in components or other data structures.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct AssetHandle {
    /// The unique ID of this asset in the cache
    id: AssetId,
    /// The file path this asset was loaded from
    path: PathBuf,
}

impl AssetHandle {
    /// Get the asset ID
    pub fn id(&self) -> AssetId {
        self.id
    }

    /// Get the file path this asset was loaded from
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Trait for types that can be loaded from files and cached.
///
/// Implement this trait for any type you want to manage through the asset cache.
/// The type must be `Send + Sync + 'static` to allow shared access across threads.
pub trait Asset: Sized + Send + Sync + 'static {
    /// Load this asset from the given file path.
    ///
    /// # Errors
    ///
    /// Returns an [`AssetError`] if the file cannot be read or parsed.
    fn load_from_file(path: &Path) -> Result<Self, AssetError>;
}

/// Internal storage for a cached asset.
///
/// Stores the type-erased asset data along with metadata for dependency
/// tracking and hot-reload support.
struct CachedEntry {
    /// The asset data, type-erased behind `Arc<dyn Any + Send + Sync>`
    data: Arc<dyn Any + Send + Sync>,
    /// The file path this asset was loaded from
    path: PathBuf,
    /// When the asset was last loaded (used for hot-reload change detection)
    load_time: SystemTime,
    /// Names of scenes or systems that depend on this asset
    dependents: Vec<String>,
}

/// A type-erased asset cache with hot-reload and dependency tracking.
///
/// The cache stores assets as `Arc<dyn Any + Send + Sync>` internally,
/// allowing it to hold assets of any type that implements [`Asset`].
/// Assets are indexed by both their [`AssetId`] and file path.
///
/// # Features
///
/// - **Deduplication**: Loading the same file path twice returns the same handle
/// - **Reference counting**: Asset data is wrapped in `Arc` for shared access
/// - **Dependency tracking**: Tracks which scenes depend on which assets
/// - **Garbage collection**: Removes assets with no dependents
/// - **Hot reload**: Detects file changes and reloads modified assets
pub struct AssetCache {
    /// Asset data indexed by ID
    assets: HashMap<AssetId, CachedEntry>,
    /// Reverse index from file path to asset ID (for deduplication)
    path_index: HashMap<PathBuf, AssetId>,
    /// Counter for generating unique asset IDs
    next_id: u64,
    /// Whether hot-reload file watching is enabled
    watch_for_changes: bool,
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetCache {
    /// Create a new empty asset cache.
    ///
    /// Hot-reload watching is disabled by default. Use [`set_watch_for_changes`](Self::set_watch_for_changes)
    /// to enable it.
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            path_index: HashMap::new(),
            next_id: 1, // Start at 1; 0 is reserved as "no asset"
            watch_for_changes: false,
        }
    }

    /// Load an asset from the given file path, or return the cached handle if
    /// already loaded.
    ///
    /// If the file has already been loaded, this returns the existing handle
    /// without re-reading the file. Otherwise, it calls `T::load_from_file`
    /// and caches the result.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The asset type to load. Must implement [`Asset`].
    ///
    /// # Errors
    ///
    /// Returns an [`AssetError`] if the file cannot be loaded.
    pub fn load<T: Asset>(&mut self, path: impl AsRef<Path>) -> Result<AssetHandle, AssetError> {
        let path = path.as_ref().to_path_buf();

        // Check if already cached (deduplication by path)
        if let Some(&id) = self.path_index.get(&path) {
            return Ok(AssetHandle { id, path });
        }

        // Load from file
        let data = T::load_from_file(&path)?;
        let arc_data: Arc<dyn Any + Send + Sync> = Arc::new(data);

        let id = self.next_id;
        self.next_id += 1;

        let entry = CachedEntry {
            data: arc_data,
            path: path.clone(),
            load_time: SystemTime::now(),
            dependents: Vec::new(),
        };

        self.assets.insert(id, entry);
        self.path_index.insert(path.clone(), id);

        Ok(AssetHandle { id, path })
    }

    /// Retrieve a cached asset by its handle, downcasting to the requested type.
    ///
    /// Returns `None` if the handle is invalid (asset was removed) or if the
    /// stored type does not match `T`.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The expected asset type. Must match the type used when loading.
    pub fn get<T: Asset>(&self, handle: &AssetHandle) -> Option<Arc<T>> {
        let entry = self.assets.get(&handle.id)?;
        // Downcast from Arc<dyn Any + Send + Sync> to Arc<T>
        entry.data.clone().downcast::<T>().ok()
    }

    /// Add a named dependent (e.g., a scene name) to an asset.
    ///
    /// This is used for dependency tracking: when a scene loads an asset,
    /// it registers itself as a dependent. The garbage collector will not
    /// remove assets that have dependents.
    pub fn add_dependent(&mut self, handle: &AssetHandle, scene_name: &str) {
        if let Some(entry) = self.assets.get_mut(&handle.id) {
            if !entry.dependents.contains(&scene_name.to_string()) {
                entry.dependents.push(scene_name.to_string());
            }
        }
    }

    /// Remove a named dependent from an asset.
    ///
    /// Call this when a scene is unloaded to release its claim on the asset.
    /// If no dependents remain, the asset becomes eligible for garbage collection.
    pub fn remove_dependent(&mut self, handle: &AssetHandle, scene_name: &str) {
        if let Some(entry) = self.assets.get_mut(&handle.id) {
            entry.dependents.retain(|d| d != scene_name);
        }
    }

    /// Enable or disable hot-reload file change watching.
    ///
    /// When enabled, [`check_hot_reload`](Self::check_hot_reload) will compare
    /// file modification times against cached load times and reload changed assets.
    pub fn set_watch_for_changes(&mut self, enabled: bool) {
        self.watch_for_changes = enabled;
    }

    /// Check whether hot-reload watching is enabled.
    pub fn is_watching_for_changes(&self) -> bool {
        self.watch_for_changes
    }

    /// Check for file changes and reload modified assets.
    ///
    /// Iterates over all cached assets, compares their file modification time
    /// against the cached load time, and reloads any that have changed. Returns
    /// handles to all assets that were successfully reloaded.
    ///
    /// This is a no-op if [`set_watch_for_changes`](Self::set_watch_for_changes)
    /// has not been enabled.
    ///
    /// # Type Parameters
    ///
    /// - `T`: The asset type to check. Only assets of this type will be checked.
    ///   Call this once per asset type you want to hot-reload.
    pub fn check_hot_reload<T: Asset>(&mut self) -> Vec<AssetHandle> {
        if !self.watch_for_changes {
            return Vec::new();
        }

        let mut reloaded = Vec::new();

        // Collect IDs to check (avoid borrowing self during iteration)
        let ids_and_paths: Vec<(AssetId, PathBuf, SystemTime)> = self
            .assets
            .iter()
            .map(|(&id, entry)| (id, entry.path.clone(), entry.load_time))
            .collect();

        for (id, path, load_time) in ids_and_paths {
            // Check if the file has been modified since we loaded it
            let modified = match std::fs::metadata(&path) {
                Ok(meta) => match meta.modified() {
                    Ok(time) => time,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            if modified > load_time {
                // File has changed; try to reload
                match T::load_from_file(&path) {
                    Ok(new_data) => {
                        let arc_data: Arc<dyn Any + Send + Sync> = Arc::new(new_data);
                        if let Some(entry) = self.assets.get_mut(&id) {
                            entry.data = arc_data;
                            entry.load_time = SystemTime::now();
                        }
                        reloaded.push(AssetHandle {
                            id,
                            path: path.clone(),
                        });
                        log::info!("Hot-reloaded asset: {}", path.display());
                    }
                    Err(err) => {
                        log::warn!("Failed to hot-reload asset {}: {}", path.display(), err);
                    }
                }
            }
        }

        reloaded
    }

    /// Run garbage collection, removing assets with no dependents.
    ///
    /// Returns the number of assets that were removed.
    pub fn gc(&mut self) -> usize {
        // Collect IDs of assets with no dependents
        let to_remove: Vec<AssetId> = self
            .assets
            .iter()
            .filter(|(_, entry)| entry.dependents.is_empty())
            .map(|(&id, _)| id)
            .collect();

        let count = to_remove.len();

        for id in &to_remove {
            if let Some(entry) = self.assets.remove(id) {
                self.path_index.remove(&entry.path);
            }
        }

        count
    }

    /// Get the number of assets currently in the cache.
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    /// Get the file path associated with an asset handle.
    pub fn handle_path<'a>(&self, handle: &'a AssetHandle) -> &'a Path {
        &handle.path
    }

    /// Check if an asset with the given handle is still in the cache.
    pub fn contains(&self, handle: &AssetHandle) -> bool {
        self.assets.contains_key(&handle.id)
    }

    /// Get the list of dependents for an asset.
    ///
    /// Returns `None` if the handle is invalid.
    pub fn dependents(&self, handle: &AssetHandle) -> Option<&[String]> {
        self.assets.get(&handle.id).map(|e| e.dependents.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// A simple test asset: just holds a string loaded from a file.
    #[derive(Debug, Clone, PartialEq)]
    struct TextAsset {
        content: String,
    }

    impl Asset for TextAsset {
        fn load_from_file(path: &Path) -> Result<Self, AssetError> {
            let content = fs::read_to_string(path)?;
            Ok(TextAsset { content })
        }
    }

    /// A different asset type for testing type mismatch.
    #[derive(Debug, Clone, PartialEq)]
    struct NumberAsset {
        value: u64,
    }

    impl Asset for NumberAsset {
        fn load_from_file(path: &Path) -> Result<Self, AssetError> {
            let content = fs::read_to_string(path)?;
            let value: u64 = content
                .trim()
                .parse()
                .map_err(|e: std::num::ParseIntError| AssetError::Parse(e.to_string()))?;
            Ok(NumberAsset { value })
        }
    }

    /// Helper to create a temp file with given content, returning its path.
    fn create_temp_file(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("rust4d_asset_tests");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    /// Helper to clean up temp files after tests.
    fn cleanup_temp_file(path: &Path) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_new_cache_is_empty() {
        let cache = AssetCache::new();
        assert_eq!(cache.asset_count(), 0);
        assert!(!cache.is_watching_for_changes());
    }

    #[test]
    fn test_default_cache_is_empty() {
        let cache = AssetCache::default();
        assert_eq!(cache.asset_count(), 0);
    }

    #[test]
    fn test_load_and_retrieve() {
        let path = create_temp_file("test_load.txt", "hello world");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        assert_eq!(cache.asset_count(), 1);
        assert_eq!(handle.id(), 1);
        assert_eq!(handle.path(), path);

        let data = cache.get::<TextAsset>(&handle).unwrap();
        assert_eq!(data.content, "hello world");

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_duplicate_path_returns_same_handle() {
        let path = create_temp_file("test_dup.txt", "data");

        let mut cache = AssetCache::new();
        let handle1 = cache.load::<TextAsset>(&path).unwrap();
        let handle2 = cache.load::<TextAsset>(&path).unwrap();

        // Should be the same handle (same id and path)
        assert_eq!(handle1, handle2);
        // Should still be only one asset in cache
        assert_eq!(cache.asset_count(), 1);

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_different_paths_get_different_handles() {
        let path1 = create_temp_file("test_diff1.txt", "aaa");
        let path2 = create_temp_file("test_diff2.txt", "bbb");

        let mut cache = AssetCache::new();
        let handle1 = cache.load::<TextAsset>(&path1).unwrap();
        let handle2 = cache.load::<TextAsset>(&path2).unwrap();

        assert_ne!(handle1, handle2);
        assert_eq!(cache.asset_count(), 2);

        let data1 = cache.get::<TextAsset>(&handle1).unwrap();
        let data2 = cache.get::<TextAsset>(&handle2).unwrap();
        assert_eq!(data1.content, "aaa");
        assert_eq!(data2.content, "bbb");

        cleanup_temp_file(&path1);
        cleanup_temp_file(&path2);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let mut cache = AssetCache::new();
        let result = cache.load::<TextAsset>("/nonexistent/path/to/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_with_wrong_type_returns_none() {
        let path = create_temp_file("test_wrongtype.txt", "42");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        // Loaded as TextAsset, trying to get as NumberAsset should return None
        let result = cache.get::<NumberAsset>(&handle);
        assert!(result.is_none());

        // Getting as TextAsset should work
        let result = cache.get::<TextAsset>(&handle);
        assert!(result.is_some());

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_get_with_invalid_handle_returns_none() {
        let cache = AssetCache::new();
        let fake_handle = AssetHandle {
            id: 999,
            path: PathBuf::from("fake.txt"),
        };
        let result = cache.get::<TextAsset>(&fake_handle);
        assert!(result.is_none());
    }

    #[test]
    fn test_dependent_tracking_add() {
        let path = create_temp_file("test_dep_add.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        cache.add_dependent(&handle, "scene_main");
        cache.add_dependent(&handle, "scene_level1");

        let deps = cache.dependents(&handle).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"scene_main".to_string()));
        assert!(deps.contains(&"scene_level1".to_string()));

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_dependent_tracking_no_duplicate() {
        let path = create_temp_file("test_dep_nodup.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        cache.add_dependent(&handle, "scene_main");
        cache.add_dependent(&handle, "scene_main"); // duplicate

        let deps = cache.dependents(&handle).unwrap();
        assert_eq!(deps.len(), 1);

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_dependent_tracking_remove() {
        let path = create_temp_file("test_dep_remove.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        cache.add_dependent(&handle, "scene_main");
        cache.add_dependent(&handle, "scene_level1");
        cache.remove_dependent(&handle, "scene_main");

        let deps = cache.dependents(&handle).unwrap();
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&"scene_level1".to_string()));

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_gc_removes_unused_assets() {
        let path1 = create_temp_file("test_gc1.txt", "aaa");
        let path2 = create_temp_file("test_gc2.txt", "bbb");

        let mut cache = AssetCache::new();
        let _handle1 = cache.load::<TextAsset>(&path1).unwrap();
        let _handle2 = cache.load::<TextAsset>(&path2).unwrap();

        assert_eq!(cache.asset_count(), 2);

        // No dependents on either, so gc should remove both
        let removed = cache.gc();
        assert_eq!(removed, 2);
        assert_eq!(cache.asset_count(), 0);

        cleanup_temp_file(&path1);
        cleanup_temp_file(&path2);
    }

    #[test]
    fn test_gc_preserves_assets_with_dependents() {
        let path1 = create_temp_file("test_gc_keep1.txt", "aaa");
        let path2 = create_temp_file("test_gc_keep2.txt", "bbb");

        let mut cache = AssetCache::new();
        let handle1 = cache.load::<TextAsset>(&path1).unwrap();
        let _handle2 = cache.load::<TextAsset>(&path2).unwrap();

        // Only add a dependent to handle1
        cache.add_dependent(&handle1, "scene_main");

        assert_eq!(cache.asset_count(), 2);

        let removed = cache.gc();
        assert_eq!(removed, 1); // Only handle2 (no dependents) should be removed
        assert_eq!(cache.asset_count(), 1);

        // handle1 should still be accessible
        assert!(cache.contains(&handle1));
        let data = cache.get::<TextAsset>(&handle1).unwrap();
        assert_eq!(data.content, "aaa");

        cleanup_temp_file(&path1);
        cleanup_temp_file(&path2);
    }

    #[test]
    fn test_gc_after_removing_all_dependents() {
        let path = create_temp_file("test_gc_after_remove.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        cache.add_dependent(&handle, "scene_main");
        assert_eq!(cache.gc(), 0); // Has dependent, should not be collected

        cache.remove_dependent(&handle, "scene_main");
        assert_eq!(cache.gc(), 1); // No dependents now, should be collected

        assert_eq!(cache.asset_count(), 0);

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_handle_path() {
        let path = create_temp_file("test_handle_path.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        assert_eq!(cache.handle_path(&handle), path.as_path());

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_contains() {
        let path = create_temp_file("test_contains.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        assert!(cache.contains(&handle));

        // GC removes it (no dependents)
        cache.gc();
        assert!(!cache.contains(&handle));

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_set_watch_for_changes() {
        let mut cache = AssetCache::new();
        assert!(!cache.is_watching_for_changes());

        cache.set_watch_for_changes(true);
        assert!(cache.is_watching_for_changes());

        cache.set_watch_for_changes(false);
        assert!(!cache.is_watching_for_changes());
    }

    #[test]
    fn test_hot_reload_disabled_returns_empty() {
        let mut cache = AssetCache::new();
        // watch_for_changes is false by default
        let reloaded = cache.check_hot_reload::<TextAsset>();
        assert!(reloaded.is_empty());
    }

    #[test]
    fn test_hot_reload_detects_change() {
        let path = create_temp_file("test_hot_reload.txt", "original");

        let mut cache = AssetCache::new();
        cache.set_watch_for_changes(true);
        let handle = cache.load::<TextAsset>(&path).unwrap();

        // Verify original content
        let data = cache.get::<TextAsset>(&handle).unwrap();
        assert_eq!(data.content, "original");

        // Wait a moment to ensure file system timestamp differs
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Modify the file
        fs::write(&path, "modified").unwrap();

        // Check for hot reload
        let reloaded = cache.check_hot_reload::<TextAsset>();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].id(), handle.id());

        // Verify updated content
        let data = cache.get::<TextAsset>(&handle).unwrap();
        assert_eq!(data.content, "modified");

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_hot_reload_no_change() {
        let path = create_temp_file("test_hot_nochange.txt", "stable");

        let mut cache = AssetCache::new();
        cache.set_watch_for_changes(true);
        let _handle = cache.load::<TextAsset>(&path).unwrap();

        // Without modifying the file, hot reload should find nothing
        let reloaded = cache.check_hot_reload::<TextAsset>();
        assert!(reloaded.is_empty());

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_number_asset_load() {
        let path = create_temp_file("test_number.txt", "42");

        let mut cache = AssetCache::new();
        let handle = cache.load::<NumberAsset>(&path).unwrap();

        let data = cache.get::<NumberAsset>(&handle).unwrap();
        assert_eq!(data.value, 42);

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_number_asset_parse_error() {
        let path = create_temp_file("test_number_bad.txt", "not a number");

        let mut cache = AssetCache::new();
        let result = cache.load::<NumberAsset>(&path);
        assert!(result.is_err());

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_asset_ids_increment() {
        let path1 = create_temp_file("test_id1.txt", "a");
        let path2 = create_temp_file("test_id2.txt", "b");
        let path3 = create_temp_file("test_id3.txt", "c");

        let mut cache = AssetCache::new();
        let h1 = cache.load::<TextAsset>(&path1).unwrap();
        let h2 = cache.load::<TextAsset>(&path2).unwrap();
        let h3 = cache.load::<TextAsset>(&path3).unwrap();

        assert_eq!(h1.id(), 1);
        assert_eq!(h2.id(), 2);
        assert_eq!(h3.id(), 3);

        cleanup_temp_file(&path1);
        cleanup_temp_file(&path2);
        cleanup_temp_file(&path3);
    }

    #[test]
    fn test_handle_clone_equality() {
        let path = create_temp_file("test_clone.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();
        let cloned = handle.clone();

        assert_eq!(handle, cloned);
        assert_eq!(handle.id(), cloned.id());
        assert_eq!(handle.path(), cloned.path());

        cleanup_temp_file(&path);
    }

    #[test]
    fn test_dependents_on_invalid_handle() {
        let cache = AssetCache::new();
        let fake_handle = AssetHandle {
            id: 999,
            path: PathBuf::from("fake.txt"),
        };
        assert!(cache.dependents(&fake_handle).is_none());
    }

    #[test]
    fn test_add_dependent_on_invalid_handle_is_noop() {
        let mut cache = AssetCache::new();
        let fake_handle = AssetHandle {
            id: 999,
            path: PathBuf::from("fake.txt"),
        };
        // Should not panic
        cache.add_dependent(&fake_handle, "scene");
        cache.remove_dependent(&fake_handle, "scene");
    }

    #[test]
    fn test_gc_on_empty_cache() {
        let mut cache = AssetCache::new();
        assert_eq!(cache.gc(), 0);
    }

    #[test]
    fn test_multiple_dependents_and_gc() {
        let path = create_temp_file("test_multi_dep.txt", "data");

        let mut cache = AssetCache::new();
        let handle = cache.load::<TextAsset>(&path).unwrap();

        cache.add_dependent(&handle, "scene_a");
        cache.add_dependent(&handle, "scene_b");
        cache.add_dependent(&handle, "scene_c");

        // Remove one dependent at a time
        cache.remove_dependent(&handle, "scene_a");
        assert_eq!(cache.gc(), 0); // Still has 2 dependents

        cache.remove_dependent(&handle, "scene_b");
        assert_eq!(cache.gc(), 0); // Still has 1 dependent

        cache.remove_dependent(&handle, "scene_c");
        assert_eq!(cache.gc(), 1); // All dependents removed

        cleanup_temp_file(&path);
    }
}
