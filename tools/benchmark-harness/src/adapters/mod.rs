//! Framework adapter implementations

pub mod external;
pub mod kreuzberg;
pub mod native;
pub mod node;
pub mod python;
pub mod ruby;
pub mod subprocess;

pub use external::{
    create_docling_adapter, create_docling_batch_adapter, create_markitdown_adapter, create_tika_batch_adapter,
    create_tika_sync_adapter, create_unstructured_adapter,
};
pub use kreuzberg::{
    create_csharp_sync_adapter, create_go_batch_adapter, create_go_sync_adapter, create_java_sync_adapter,
    create_node_async_adapter, create_node_batch_adapter, create_python_async_adapter, create_python_batch_adapter,
    create_python_sync_adapter, create_ruby_batch_adapter, create_ruby_sync_adapter,
};
pub use native::NativeAdapter;
pub use node::NodeAdapter;
pub use python::PythonAdapter;
pub use ruby::RubyAdapter;
pub use subprocess::SubprocessAdapter;
