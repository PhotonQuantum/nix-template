use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::sync::{Arc, Mutex};

use eyre::eyre;
use log::info;

use crate::{Result, LOCK_FORMAT};

static GLOBAL_STORE: Mutex<Option<Arc<dyn Store + Send + Sync>>> = Mutex::new(None);

pub fn set_global_store(store: Arc<dyn Store + Send + Sync>) {
    *GLOBAL_STORE.lock().unwrap() = Some(store);
}

pub fn delete_global_store() {
    *GLOBAL_STORE.lock().unwrap() = None;
}

pub fn get_global_store() -> Arc<dyn Store + Send + Sync> {
    GLOBAL_STORE.lock().unwrap().clone().unwrap()
}

pub trait Store {
    fn try_get_cached(&self, path: &[String]) -> Option<String>;
    fn put_cache(&self, path: &[String], value: String);
}

#[derive(Clone)]
pub struct FileStore {
    file: Arc<File>,
    data: Arc<Mutex<serde_json::Value>>,
}

impl FileStore {
    /// Create a new store from a file.
    ///
    /// `load` specifies whether to load the file into memory.
    pub fn with(file: File, load: bool) -> Result<Self> {
        fn check_version(data: &serde_json::Value) -> bool {
            data.get("version")
                .and_then(serde_json::Value::as_u64)
                .map_or(false, |v| v == LOCK_FORMAT as u64)
        }
        fn is_empty(e: &serde_json::Error) -> bool {
            e.is_eof() && e.line() == 1 && e.column() == 0
        }
        if load {
            info!("Loading cache...");
            match serde_json::from_reader(&file) {
                Ok(data) if check_version(&data) => Ok(Self {
                    file: Arc::new(file),
                    data: Arc::new(Mutex::new(data)),
                }),
                Err(e) if !is_empty(&e) => Err(e.into()),
                // This is the case when the file is empty, or the format version doesn't match.
                // We just create an empty object.
                _ => Ok(Self {
                    file: Arc::new(file),
                    data: Arc::new(Mutex::new(serde_json::json!({ "version": LOCK_FORMAT }))),
                }),
            }
        } else {
            // We don't load the file, so we just create an empty object.
            Ok(Self {
                file: Arc::new(file),
                data: Arc::new(Mutex::new(serde_json::json!({ "version": LOCK_FORMAT }))),
            })
        }
    }
    pub fn persist(self) -> Result<()> {
        let mut file = Arc::try_unwrap(self.file)
            .map_err(|_| eyre!("FileStore instance is not unique (multiple references)"))?;
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        serde_json::to_writer_pretty(file, &*self.data.lock().unwrap())?;
        Ok(())
    }
}

impl Store for FileStore {
    fn try_get_cached(&self, path: &[String]) -> Option<String> {
        info!("cache access: {:?}", path);
        let mut data = self.data.lock().unwrap();
        let item =
            path[..path.len() - 1]
                .iter()
                .fold(data.as_object_mut().unwrap(), |data, key| {
                    data.entry(key)
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                        .unwrap()
                });
        item.get(path.last().unwrap())
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
    }

    fn put_cache(&self, path: &[String], value: String) {
        info!("cache put: {:?} = {}", path, value);
        let mut data = self.data.lock().unwrap();
        let item =
            path[..path.len() - 1]
                .iter()
                .fold(data.as_object_mut().unwrap(), |data, key| {
                    data.entry(key)
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                        .unwrap()
                });
        item.insert(
            path.last().unwrap().clone(),
            serde_json::Value::String(value),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use log::info;
    use minijinja::Environment;

    use nix_template_macros::helper_func;

    use crate::store::{FileStore, Store};
    use crate::Result;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[derive(Clone, Default)]
    pub struct MemoryStore(Arc<Mutex<HashMap<Vec<String>, String>>>);

    impl MemoryStore {
        pub fn new(map: HashMap<Vec<String>, String>) -> Self {
            Self(Arc::new(Mutex::new(map)))
        }
    }

    impl Store for MemoryStore {
        fn try_get_cached(&self, path: &[String]) -> Option<String> {
            info!("cache access: {:?}", path);
            self.0.lock().unwrap().get(path).cloned()
        }

        fn put_cache(&self, path: &[String], value: String) {
            info!("cache put: {:?} = {}", path, value);
            self.0.lock().unwrap().insert(path.to_vec(), value);
        }
    }

    fn with_store(store: impl Store + Send + Sync + 'static, f: impl FnOnce()) {
        let _lock = TEST_LOCK.lock().unwrap();
        let store = Arc::new(store);
        super::set_global_store(store);
        f();
        super::delete_global_store();
    }

    #[helper_func(cached = f)]
    fn f_hole(_a: usize, _b: &str) -> Result<String> {
        unreachable!()
    }

    #[allow(clippy::unnecessary_wraps)]
    #[helper_func(cached)]
    fn f(a: usize, b: &str) -> Result<String> {
        Ok(format!("{}{}", a, b))
    }

    #[allow(clippy::unnecessary_wraps)]
    #[helper_func(cached)]
    fn g() -> Result<String> {
        Ok("g".to_string())
    }

    #[allow(clippy::unnecessary_wraps)]
    #[helper_func(cached = g)]
    fn g_hole() -> Result<String> {
        Ok("g".to_string())
    }

    #[test]
    fn must_resolve_from_cache() {
        let store = MemoryStore::new(maplit::hashmap! {
            vec!["f".to_string(), "1".to_string(), "foo".to_string()] => "1foo".to_string(),
            vec!["f".to_string(), "2".to_string(), "bar".to_string()] => "2bar".to_string(),
        });

        let mut env = Environment::new();
        env.add_function("f", f_hole);
        with_store(store, || {
            assert_eq!(
                env.render_str("{{ f(1, 'foo') }} {{ f(2, 'bar') }}", minijinja::context!())
                    .unwrap(),
                "1foo 2bar"
            );
        });
    }

    #[test]
    fn must_cache_to_file() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let store = FileStore::with(temp_file.reopen().unwrap(), true).unwrap();

        let mut env = Environment::new();
        env.add_function("f", f);
        env.add_function("g", g);
        with_store(store.clone(), || {
            assert_eq!(
                env.render_str(
                    "{{ f(1, 'foo') }} {{ f(2, '/bar') }} {{ g() }}",
                    minijinja::context!()
                )
                .unwrap(),
                "1foo 2/bar g"
            );
        });
        store.persist().unwrap();

        eprintln!("{}", std::fs::read_to_string(temp_file.path()).unwrap());

        let store = FileStore::with(temp_file.reopen().unwrap(), true).unwrap();

        let mut env = Environment::new();
        env.add_function("f", f_hole);
        env.add_function("g", g_hole);
        with_store(store, || {
            assert_eq!(
                env.render_str(
                    "{{ f(1, 'foo') }} {{ f(2, '/bar') }} {{ g() }}",
                    minijinja::context!()
                )
                .unwrap(),
                "1foo 2/bar g"
            );
        });
    }
}
