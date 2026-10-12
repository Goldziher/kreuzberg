//! `--content-format` / `--output-format`, `--include-structure`, and `--jupyter-cell-rendering`.

use xberg::{ExtractionConfig, JupyterCellRendering};

use super::ExtractionOverrides;

/// Which parts of a Jupyter code cell to render during extraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum JupyterCellRenderingArg {
    /// Render only the code source; omit saved outputs.
    Source,
    /// Render only the saved cell outputs; omit the code source.
    Outputs,
    /// Render both the code source and the saved outputs (default).
    Both,
}

impl From<JupyterCellRenderingArg> for JupyterCellRendering {
    fn from(arg: JupyterCellRenderingArg) -> Self {
        match arg {
            JupyterCellRenderingArg::Source => JupyterCellRendering::Source,
            JupyterCellRenderingArg::Outputs => JupyterCellRendering::Outputs,
            JupyterCellRenderingArg::Both => JupyterCellRendering::Both,
        }
    }
}

impl ExtractionOverrides {
    pub(super) fn apply_output_format(&self, config: &mut ExtractionConfig) {
        let final_format = self.content_format.or_else(|| {
            if self.output_format.is_some() {
                tracing::warn!("'--output-format' is deprecated, use '--content-format' instead");
            }
            self.output_format
        });

        if let Some(content_fmt) = final_format {
            config.output_format = content_fmt.into();
        }
    }

    pub(super) fn apply_include_structure(&self, config: &mut ExtractionConfig) {
        if let Some(flag) = self.include_structure {
            config.include_document_structure = flag;
        }
    }

    pub(super) fn apply_strict_parse(&self, config: &mut ExtractionConfig) {
        if let Some(flag) = self.strict_parse {
            config.strict_parse = flag;
        }
    }

    pub(super) fn apply_jupyter_cell_rendering(&self, config: &mut ExtractionConfig) {
        if let Some(rendering) = self.jupyter_cell_rendering {
            config.jupyter_cell_rendering = rendering.into();
        }
    }
}
