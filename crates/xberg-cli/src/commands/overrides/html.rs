//! `--html-*` styled-output overrides.

use xberg::ExtractionConfig;

use super::ExtractionOverrides;

impl ExtractionOverrides {
    #[allow(unused_variables)]
    pub(super) fn apply_html_styled(&self, config: &mut ExtractionConfig) {
        #[cfg(feature = "html")]
        {
            let has_flag = self.html_theme.is_some()
                || self.html_css.is_some()
                || self.html_css_file.is_some()
                || self.html_class_prefix.is_some()
                || self.html_no_embed_css;

            if has_flag {
                config.output_format = xberg::OutputFormat::Html;

                let mut html_cfg = config.html_output.clone().unwrap_or_default();

                if let Some(ref theme_str) = self.html_theme {
                    html_cfg.theme = match theme_str.to_lowercase().as_str() {
                        "github" => xberg::HtmlTheme::GitHub,
                        "dark" => xberg::HtmlTheme::Dark,
                        "light" => xberg::HtmlTheme::Light,
                        "unstyled" => xberg::HtmlTheme::Unstyled,
                        _ => xberg::HtmlTheme::Default,
                    };
                }

                if let Some(ref css) = self.html_css {
                    html_cfg.css = Some(css.clone());
                }

                if let Some(ref path) = self.html_css_file {
                    html_cfg.css_file = Some(path.clone());
                }

                if let Some(ref prefix) = self.html_class_prefix {
                    html_cfg.class_prefix = prefix.clone();
                }

                if self.html_no_embed_css {
                    html_cfg.embed_css = false;
                }

                config.html_output = Some(html_cfg);
            }
        }
    }
}
