//! `src/cache.rs` — cache LRU de textures GPU, chargement asynchrone des images
//! et validation stricte des chemins fournis par le démon ZeedeFi.
//!
//! Toute I/O disque passe par ce module : la validation rejette les chemins
//! relatifs, les traversées `..` et les symlinks qui sortent des racines
//! autorisées (défense contre l'exécution de code arbitraire via les données
//! du démon).

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Component, Path, PathBuf};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::imageops::FilterType;
use image::RgbaImage;
use lru::LruCache;
use thiserror::Error;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};


/// Capacité du cache LRU (nombre de textures).
pub const CACHE_CAPACITY: usize = 20;

/// Largeur maximale des fonds d'écran décodés (le fond est étiré à l'écran).
pub const MAX_BACKGROUND_WIDTH: u32 = 1920;

/// Sigma du flou gaussien appliqué aux fonds d'écran (en pixels).
pub const BACKGROUND_BLUR_SIGMA: f32 = 4.0;

/// Variable d'environnement listant les racines autorisées pour les assets.
pub const ENV_ASSET_ROOTS: &str = "ZEEDEFI_ASSET_ROOTS";

/// Erreurs liées à la validation et au chargement des assets.
#[derive(Debug, Error, Clone)]
pub enum CacheError {
    #[error("chemin relatif refusé: {0}")]
    RelativePath(PathBuf),
    #[error("traversée de parent (..) interdite: {0}")]
    ParentTraversal(PathBuf),
    #[error("chemin hors des racines autorisées: {0}")]
    OutsideRoots(PathBuf),
    #[error("asset introuvable: {0}")]
    NotFound(PathBuf),
    #[error("erreur de décodage d'image: {0}")]
    Decode(String),
    #[error("erreur d'E/S: {0}")]
    Io(String),
}

/// Validateur de chemins : résout les symlinks et vérifie l'appartenance à
/// l'une des racines autorisées.
#[derive(Debug, Clone)]
pub struct PathValidator {
    roots: Vec<PathBuf>,
}

impl PathValidator {
    /// Construit un validateur à partir des racines par défaut et de la
    /// variable d'environnement `ZEEDEFI_ASSET_ROOTS` (séparateur `:`).
    pub fn from_env() -> Self {
        let env_roots = std::env::var_os(ENV_ASSET_ROOTS)
            .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut roots = env_roots;
        roots.extend(default_roots());
        roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
        let canonicalized: Vec<PathBuf> = roots
            .into_iter()
            .filter(|path| path.is_absolute())
            .filter_map(|path| path.canonicalize().ok())
            .collect();
        Self {
            roots: canonicalized,
        }
    }

    /// Valide un chemin et retourne sa forme canonique.
    pub fn validate(&self, path: &Path) -> Result<PathBuf, CacheError> {
        if !path.is_absolute() {
            return Err(CacheError::RelativePath(path.to_path_buf()));
        }
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(CacheError::ParentTraversal(path.to_path_buf()));
        }
        let canonical = path
            .canonicalize()
            .map_err(|_| CacheError::NotFound(path.to_path_buf()))?;
        if !self.roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(CacheError::OutsideRoots(canonical));
        }
        Ok(canonical)
    }
}

fn default_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/share/zeedefi"),
        PathBuf::from("/var/lib/zeedefi"),
        PathBuf::from("/var/cache/zeedefi"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/share/zeedefi"));
        roots.push(home.join(".cache/zeedefi"));
        roots.push(home.join(".cache/zeedefi-launcher"));
    }
    roots
}

/// Type d'asset à charger (influe sur le traitement appliqué).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetKind {
    Cover,
    Background,
    BlurredBackground,
}

impl AssetKind {
    pub(crate) fn key_tag(self) -> &'static str {
        match self {
            AssetKind::Cover => "cover",
            AssetKind::Background => "bg",
            AssetKind::BlurredBackground => "bgblur",
        }
    }
}

/// Résultat d'un chargement asynchrone, envoyé au thread UI via canal.
#[derive(Debug)]
pub struct LoadResult {
    pub key: String,
    pub image: Option<ColorImage>,
    pub bytes: usize,
    pub error: Option<String>,
}

/// Entrée du cache LRU.
#[derive(Clone)]
struct CacheEntry {
    handle: TextureHandle,
    bytes: usize,
}

/// Cache LRU de textures GPU.
pub struct TextureCache {
    entries: LruCache<String, CacheEntry>,
    inflight: HashMap<String, AssetKind>,
}

impl std::fmt::Debug for TextureCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TextureCache")
            .field("entries", &self.entries.len())
            .field("inflight", &self.inflight.len())
            .finish()
    }
}

impl TextureCache {
    pub fn new() -> Self {
        let capacity = NonZeroUsize::new(CACHE_CAPACITY).unwrap_or(NonZeroUsize::MIN);
        Self {
            entries: LruCache::new(capacity),
            inflight: HashMap::new(),
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains(key)
    }

    pub fn is_inflight(&self, key: &str) -> bool {
        self.inflight.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&TextureHandle> {
        self.entries.peek(key).map(|entry| &entry.handle)
    }

    pub fn mark_inflight(&mut self, key: String, kind: AssetKind) {
        self.inflight.insert(key, kind);
    }

    pub fn clear_inflight(&mut self, key: &str) {
        self.inflight.remove(key);
    }

    pub fn insert(&mut self, key: String, handle: TextureHandle, bytes: usize) {
        if let Some(evicted) = self.entries.push(key, CacheEntry { handle, bytes }) {
            debug!(evicted_key = %evicted.0, "texture évincée du cache LRU");
        }
        self.log_memory();
    }

    pub fn total_bytes(&self) -> usize {
        self.entries
            .iter()
            .map(|(_, entry)| entry.bytes)
            .sum()
    }

    fn log_memory(&self) {
        trace!(total_bytes = self.total_bytes(), entries = self.entries.len(), "cache LRU");
    }
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Chargeur asynchrone : décode les images hors du thread UI et envoie les
/// résultats sur un canal consommé par `poll`.
pub struct AssetLoader {
    handle: Handle,
    sender: mpsc::UnboundedSender<LoadResult>,
    receiver: mpsc::UnboundedReceiver<LoadResult>,
}

impl std::fmt::Debug for AssetLoader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AssetLoader").finish()
    }
}

impl AssetLoader {
    pub fn new(handle: Handle) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            handle,
            sender,
            receiver,
        }
    }

    /// Demande le chargement d'un asset si nécessaire. Retourne `true` si un
    /// chargement a effectivement été lancé.
    pub fn request(
        &self,
        cache: &mut TextureCache,
        validator: &PathValidator,
        path: &Path,
        kind: AssetKind,
    ) -> bool {
        let key = asset_key(path, kind);
        if cache.contains(&key) || cache.is_inflight(&key) {
            return false;
        }
        let validated = match validator.validate(path) {
            Ok(path) => path,
            Err(error) => {
                warn!(path = %path.display(), error = %error, "asset rejeté par le validateur");
                return false;
            }
        };
        cache.mark_inflight(key.clone(), kind);
        let sender = self.sender.clone();
        self.handle.spawn_blocking(move || {
            let result = decode_asset(&validated, kind);
            let _ = sender.send(result);
        });
        true
    }

    /// Drain les résultats disponibles et met à jour le cache (thread UI).
    pub fn poll(&mut self, context: &Context, cache: &mut TextureCache) {
        while let Ok(result) = self.receiver.try_recv() {
            cache.clear_inflight(&result.key);
            match result.image {
                Some(image) => {
                    let handle = context.load_texture(result.key.clone(), image, TextureOptions::LINEAR);
                    cache.insert(result.key, handle, result.bytes);
                }
                None => {
                    if let Some(error) = result.error {
                        warn!(key = %result.key, error = %error, "échec de chargement d'asset");
                    }
                }
            }
        }
    }
}

fn asset_key(path: &Path, kind: AssetKind) -> String {
    format!("{}:{}", kind.key_tag(), path.display())
}

fn decode_asset(path: &Path, kind: AssetKind) -> LoadResult {
    let key = asset_key(path, kind);
    let image = match image::ImageReader::open(path) {
        Ok(reader) => match reader.decode() {
            Ok(decoded) => decoded.to_rgba8(),
            Err(error) => {
                return LoadResult {
                    key,
                    image: None,
                    bytes: 0,
                    error: Some(format!("decode: {error}")),
                };
            }
        },
        Err(error) => {
            return LoadResult {
                key,
                image: None,
                bytes: 0,
                error: Some(format!("open: {error}")),
            };
        }
    };
    let (color_image, bytes) = match kind {
        AssetKind::Cover => {
            let raw = image.as_raw().to_vec();
            let size = image.dimensions();
            let color = ColorImage::from_rgba_unmultiplied([size.0 as usize, size.1 as usize], &raw);
            (color, raw.len())
        }
        AssetKind::Background => {
            let resized = downscale(&image, MAX_BACKGROUND_WIDTH);
            let raw = resized.as_raw().to_vec();
            let size = resized.dimensions();
            let color = ColorImage::from_rgba_unmultiplied([size.0 as usize, size.1 as usize], &raw);
            (color, raw.len())
        }
        AssetKind::BlurredBackground => {
            let resized = downscale(&image, MAX_BACKGROUND_WIDTH);
            let blurred = image::imageops::blur(&resized, BACKGROUND_BLUR_SIGMA);
            let raw = blurred.as_raw().to_vec();
            let size = blurred.dimensions();
            let color = ColorImage::from_rgba_unmultiplied([size.0 as usize, size.1 as usize], &raw);
            (color, raw.len())
        }
    };
    LoadResult {
        key,
        image: Some(color_image),
        bytes,
        error: None,
    }
}

fn downscale(image: &RgbaImage, max_width: u32) -> RgbaImage {
    let (width, height) = image.dimensions();
    if width <= max_width {
        return image.clone();
    }
    let scale = f64::from(max_width) / f64::from(width);
    let new_height = (f64::from(height) * scale).round().max(1.0) as u32;
    image::imageops::resize(image, max_width, new_height, FilterType::Triangle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_starts_empty() {
        let cache = TextureCache::new();
        assert_eq!(cache.total_bytes(), 0);
        assert!(!cache.contains("cover:foo"));
    }

    #[test]
    fn path_validator_rejects_relative() {
        let validator = PathValidator {
            roots: vec![PathBuf::from("/tmp")],
        };
        assert!(validator.validate(Path::new("relative/path.png")).is_err());
    }

    #[test]
    fn path_validator_rejects_parent_traversal() {
        let validator = PathValidator {
            roots: vec![PathBuf::from("/tmp")],
        };
        assert!(validator.validate(Path::new("/tmp/../etc/passwd")).is_err());
    }
}
