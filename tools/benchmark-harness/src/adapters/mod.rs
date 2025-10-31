//! Framework adapter implementations

pub mod external;
pub mod native;
pub mod node;
pub mod python;
pub mod ruby;
pub mod subprocess;

pub use external::{
    create_docling_adapter, create_extractous_python_adapter, create_markitdown_adapter, create_unstructured_adapter,
};
pub use native::NativeAdapter;
pub use node::NodeAdapter;
pub use python::PythonAdapter;
pub use ruby::RubyAdapter;
pub use subprocess::SubprocessAdapter;
