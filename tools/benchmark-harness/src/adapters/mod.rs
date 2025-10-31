//! Framework adapter implementations

pub mod native;
pub mod node;
pub mod python;
pub mod ruby;
pub mod subprocess;

pub use native::NativeAdapter;
pub use node::NodeAdapter;
pub use python::PythonAdapter;
pub use ruby::RubyAdapter;
pub use subprocess::SubprocessAdapter;
