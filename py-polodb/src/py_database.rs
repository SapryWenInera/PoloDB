use crate::doc_bson_to_py_translator::{
    bson_to_py_obj, convert_py_obj_to_document, document_to_pydict,
};
use polodb_core::bson::Document;
use polodb_core::{Collection, CollectionT, Database};
use pyo3::exceptions::PyOSError;
use pyo3::exceptions::PyRuntimeError; // Import PyRuntimeError for error handling
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::borrow::Borrow;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[pyclass]
pub struct PyCollection {
    inner: Arc<Collection<Document>>, // Use Arc for thread-safe shared ownership
}

#[pymethods]
impl PyCollection {
    pub fn name(&self) -> &str {
        self.inner.name()
    }
    pub fn insert_one(&self, doc: Py<PyDict>) -> PyResult<PyObject> {
        // Acquire the Python GIL (Global Interpreter Lock)
        Python::with_gil(|py| {
            // Now you can use `py` inside this block.

            // Example: Create a Python object or interact with the Python runtime.
            let bson_doc: Document = match convert_py_obj_to_document(doc.to_object(py).as_any()) {
                Ok(d) => d,
                Err(_) => {
                    return Err(PyRuntimeError::new_err(
                        "Failed to convert Python dict to BSON document",
                    ))
                }
            };
            // let bson_doc = convert_py_to_bson(doc);
            match self.inner.insert_one(bson_doc) {
                Ok(result) => {
                    // Create a Python object from the Rust result and return it
                    let py_inserted_id = bson_to_py_obj(py, &result.inserted_id);
                    let dict = PyDict::new_bound(py);
                    let dict_ref = dict.borrow();
                    dict_ref.set_item("inserted_id", py_inserted_id)?;
                    Ok(dict.to_object(py))

                    // Ok(Py::new(py, result)?.to_object(py))
                }
                Err(e) => {
                    // Raise a Python exception on error
                    Err(PyRuntimeError::new_err(format!("Insert error: {}", e)))
                }
            }
        })
    }
    pub fn find_one(&self, py: Python, filter: Py<PyDict>) -> PyResult<Option<PyObject>> {
        // Convert PyDict to BSON Document
        let filter_doc = convert_py_obj_to_document(filter.to_object(py).as_any())?;

        // Call the Rust method `find_one`
        match self.inner.find_one(filter_doc) {
            Ok(Some(result_doc)) => {
                // Convert BSON Document to Python Dict
                let py_result = document_to_pydict(py, result_doc).unwrap();
                Ok(Some(py_result.to_object(py)))
            }
            Ok(None) => Ok(None), // Return None if no document is found
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Find one error: {}",
                err
            ))),
        }
    }
    pub fn find(&self, py: Python, filter: Py<PyDict>) -> PyResult<Option<PyObject>> {
        // Convert PyDict to BSON Document
        let filter_doc = convert_py_obj_to_document(filter.to_object(py).as_any())?;

        // Call the Rust method `find_one`
        match self.inner.find(filter_doc).run() {
            Ok(result_doc) => {
                // Convert BSON Document to Python Dict
                let py_result: Vec<Py<PyDict>> = result_doc
                    .map(|x| document_to_pydict(py, x.unwrap()).unwrap())
                    .collect();
                // let py_result = document_to_pydict(py, result_doc).unwrap();
                Ok(Some(py_result.to_object(py)))
            }
            // Ok(None) => Ok(None), // Return None if no document is found
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Find one error: {}",
                err
            ))),
        }
    }
}
impl From<Collection<Document>> for PyCollection {
    fn from(collection: Collection<Document>) -> PyCollection {
        PyCollection {
            inner: Arc::new(collection),
        }
    }
}

#[pyclass]
pub struct PyDatabase {
    inner: Arc<Mutex<Database>>,
}

#[pymethods]
impl PyDatabase {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let db_path = Path::new(path);
        match Database::open_path(db_path) {
            Ok(db) => Ok(PyDatabase {
                inner: Arc::new(Mutex::new(db)),
            }),
            Err(e) => Err(PyOSError::new_err(e.to_string())),
        }
    }

    #[staticmethod]
    fn open_path(path: &str) -> PyResult<PyDatabase> {
        let db_path = Path::new(path);
        Database::open_path(db_path)
            .map(|db| PyDatabase {
                inner: Arc::new(Mutex::new(db)),
            })
            .map_err(|e| PyOSError::new_err(e.to_string()))
    }

    pub fn create_collection(&self, name: &str) -> PyResult<()> {
        let _ = self.inner.lock().unwrap().create_collection(name);
        Ok(())
    }

    fn collection(&self, name: &str) -> PyResult<PyCollection> {
        // Attempt to acquire the lock and fetch/create the collection
        let guard = self
            .inner
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock: {}", e)))?;
        let rust_collection = guard.collection::<Document>(name); // Assume this returns a Rust Collection

        //Convert a Rust Collection to a PyCollection
        let py_collection: PyCollection = PyCollection::from(rust_collection);
        Ok(py_collection)
    }

    pub fn list_collection_names(&self) -> PyResult<Vec<String>> {
        let collections_names = self.inner.lock().unwrap().list_collection_names();
        match collections_names {
            Ok(collection_names) => Ok(collection_names),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Error listing collection names: {}",
                e
            ))),
        }
    }

    // You can add methods here to interact with the Database
}
