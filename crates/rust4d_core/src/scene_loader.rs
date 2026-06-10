//! Async scene loading
//!
//! Provides background scene loading using threads and channels.
//! The [`SceneLoader`] spawns a worker thread that processes load requests
//! and returns results via a channel, enabling non-blocking scene loading.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;

use crate::scene::{Scene, SceneError};

/// Request to load a scene in the background
struct LoadRequest {
    /// Path to the RON scene file
    path: PathBuf,
    /// Name to assign to the loaded scene
    scene_name: String,
}

/// Result of a background scene load
pub struct LoadResult {
    /// Name assigned to this scene
    pub scene_name: String,
    /// The loaded scene or error
    pub result: Result<Scene, SceneError>,
}

/// Background scene loader using a dedicated worker thread
///
/// SceneLoader maintains a worker thread that processes scene load requests
/// asynchronously. Use [`load_async`](SceneLoader::load_async) to submit
/// load requests and [`poll`](SceneLoader::poll) or
/// [`poll_all`](SceneLoader::poll_all) to check for completed loads.
///
/// # Example
/// ```ignore
/// let loader = SceneLoader::new();
/// loader.load_async("assets/scenes/level2.ron", "Level 2");
///
/// // Later in your game loop:
/// if let Some(result) = loader.poll() {
///     match result.result {
///         Ok(scene) => { /* register scene */ }
///         Err(e) => { /* handle error */ }
///     }
/// }
/// ```
pub struct SceneLoader {
    /// Channel to send load requests to the worker thread
    sender: Sender<LoadRequest>,
    /// Channel to receive load results from the worker thread
    receiver: Receiver<LoadResult>,
}

impl SceneLoader {
    /// Create a new scene loader with a background worker thread
    ///
    /// The worker thread runs until the SceneLoader is dropped.
    pub fn new() -> Self {
        let (request_tx, request_rx) = channel::<LoadRequest>();
        let (result_tx, result_rx) = channel::<LoadResult>();

        thread::spawn(move || {
            // Worker loop: process load requests until the channel closes
            while let Ok(request) = request_rx.recv() {
                let result = Scene::load(&request.path);
                let load_result = LoadResult {
                    scene_name: request.scene_name,
                    result: result.map_err(SceneError::from),
                };
                // If the receiver is dropped, we stop
                if result_tx.send(load_result).is_err() {
                    break;
                }
            }
        });

        Self {
            sender: request_tx,
            receiver: result_rx,
        }
    }

    /// Request a scene to be loaded in the background
    ///
    /// The scene will be loaded from the given path by the worker thread.
    /// Use [`poll`](SceneLoader::poll) to check for the result.
    pub fn load_async(&self, path: impl Into<PathBuf>, scene_name: impl Into<String>) {
        let request = LoadRequest {
            path: path.into(),
            scene_name: scene_name.into(),
        };
        // If send fails, the worker thread has exited (shouldn't happen normally)
        let _ = self.sender.send(request);
    }

    /// Check if any scenes have finished loading (non-blocking)
    ///
    /// Returns `Some(LoadResult)` if a scene has completed loading,
    /// or `None` if no results are available yet.
    pub fn poll(&self) -> Option<LoadResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Collect all completed loads (non-blocking)
    ///
    /// Returns a vector of all load results available at this moment.
    /// Returns an empty vector if no loads have completed.
    pub fn poll_all(&self) -> Vec<LoadResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.receiver.try_recv() {
            results.push(result);
        }
        results
    }
}

impl Default for SceneLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_construction() {
        // Creating a loader should not panic
        let _loader = SceneLoader::new();
    }

    #[test]
    fn test_loader_default() {
        // Default construction should not panic
        let _loader = SceneLoader::default();
    }

    #[test]
    fn test_poll_returns_none_when_empty() {
        let loader = SceneLoader::new();
        // Nothing has been submitted, so poll should return None
        assert!(loader.poll().is_none());
    }

    #[test]
    fn test_poll_all_returns_empty_when_nothing_loading() {
        let loader = SceneLoader::new();
        let results = loader.poll_all();
        assert!(results.is_empty());
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let loader = SceneLoader::new();
        loader.load_async("/nonexistent/path/scene.ron", "missing_scene");

        // Wait a bit for the worker to process
        std::thread::sleep(std::time::Duration::from_millis(100));

        let result = loader.poll();
        assert!(result.is_some());
        let load_result = result.unwrap();
        assert_eq!(load_result.scene_name, "missing_scene");
        assert!(load_result.result.is_err());
    }

    #[test]
    fn test_load_async_preserves_scene_name() {
        let loader = SceneLoader::new();
        loader.load_async("/nonexistent/file.ron", "my_custom_name");

        // Wait for processing
        std::thread::sleep(std::time::Duration::from_millis(100));

        if let Some(result) = loader.poll() {
            assert_eq!(result.scene_name, "my_custom_name");
        }
    }

    #[test]
    fn test_multiple_load_requests() {
        let loader = SceneLoader::new();
        loader.load_async("/nonexistent/a.ron", "scene_a");
        loader.load_async("/nonexistent/b.ron", "scene_b");
        loader.load_async("/nonexistent/c.ron", "scene_c");

        // Wait for all to process
        std::thread::sleep(std::time::Duration::from_millis(200));

        let results = loader.poll_all();
        assert_eq!(results.len(), 3);

        let names: Vec<&str> = results.iter().map(|r| r.scene_name.as_str()).collect();
        assert!(names.contains(&"scene_a"));
        assert!(names.contains(&"scene_b"));
        assert!(names.contains(&"scene_c"));
    }
}
