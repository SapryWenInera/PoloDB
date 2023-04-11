/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::error::DbErr;
use crate::{ClientSession, Config};
use super::db_inner::DatabaseInner;
use crate::db::collection::Collection;
use crate::metrics::Metrics;

pub(crate) static SHOULD_LOG: AtomicBool = AtomicBool::new(false);

///
/// API wrapper for Rust-level
///
/// Use [`Database::open_file`] API to open a database. A main database file will be
/// generated in the path user provided.
///
/// When you own an instance of a Database, the instance holds a file
/// descriptor of the database file. When the Database instance is dropped,
/// the handle of the file will be released.
///
/// # Collection
/// A [`Collection`] is a dataset of a kind of data.
/// You can use [`Database::create_collection`] to create a data collection.
/// To obtain an exist collection, use [`Database::collection`],
///
pub struct Database {
    inner: Arc<Mutex<DatabaseInner>>,
}

pub type DbResult<T> = Result<T, DbErr>;

impl Database {
    pub fn set_log(v: bool) {
        SHOULD_LOG.store(v, Ordering::SeqCst);
    }

    /// Return the version of package version in string.
    /// Defined in `Cargo.toml`.
    pub fn get_version() -> String {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        VERSION.into()
    }

    pub fn open_memory() -> DbResult<Database> {
        Database::open_memory_with_config(Config::default())
    }

    pub fn open_memory_with_config(config: Config) -> DbResult<Database> {
        let inner = DatabaseInner::open_memory(config)?;

        Ok(Database {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_file<P: AsRef<Path>>(path: P) -> DbResult<Database>  {
        Database::open_file_with_config(path, Config::default())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_file_with_config<P: AsRef<Path>>(path: P, config: Config) -> DbResult<Database>  {
        let inner = DatabaseInner::open_file(path.as_ref(), config)?;

        Ok(Database {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open_indexeddb(init_data: JsValue) -> DbResult<Database> {
        let config = Config::default();
        let inner = DatabaseInner::open_indexeddb(init_data, config)?;

        Ok(Database {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    /// Return the metrics object of the database
    pub fn metrics(&self) -> Metrics {
        let inner = self.inner.lock().unwrap();
        inner.metrics()
    }

    /// Creates a new collection in the database with the given `name`.
    pub fn create_collection(&self, name: &str) -> DbResult<()> {
        let mut inner = self.inner.lock()?;
        let _ = inner.create_collection(name)?;
        Ok(())
    }

    /// Creates a new collection in the database with the given `name`.
    pub fn create_collection_with_session(&self, name: &str, session: &mut ClientSession) -> DbResult<()> {
        let mut inner = self.inner.lock()?;
        let _ = inner.create_collection_internal(name, &mut session.inner)?;
        Ok(())
    }

    ///
    /// [error]: ../enum.DbErr.html
    ///
    /// Return an exist collection. If the collection is not exists,
    /// a new collection will be created.
    ///
    pub fn collection<T: Serialize>(&self, col_name: &str) -> Collection<T> {
        Collection::new(Arc::downgrade(&self.inner), col_name)
    }

    pub fn start_session(&self) -> DbResult<ClientSession> {
        let mut inner = self.inner.lock()?;
        let inner = inner.start_session()?;
        Ok(ClientSession::new(inner))
    }

    /// Gets the names of the collections in the database.
    pub fn list_collection_names(&self) -> DbResult<Vec<String>> {
        let mut inner = self.inner.lock()?;
        let mut session = inner.start_session()?;
        inner.list_collection_names_with_session(&mut session)
    }

    /// Gets the names of the collections in the database.
    pub fn list_collection_names_with_session(&self, session: &mut ClientSession) -> DbResult<Vec<String>> {
        let mut inner = self.inner.lock()?;
        inner.list_collection_names_with_session(&mut session.inner)
    }

}
