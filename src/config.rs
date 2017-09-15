use Headers;

type InternalHeaders = Vec<(String, Vec<Vec<u8>>)>;
/// A router configuration.
#[derive(Debug, Default, Clone)]
pub struct Config {
    handle_head: Option<bool>,
    extra_headers: Option<InternalHeaders>,
}

impl Config {
    /// Creates a default config without any options.
    pub fn new() -> Self {
        Config::default()
    }

    /// Creates a base config with default values explicitly set.
    ///
    /// Providing this config to a sub-router will not cause
    /// inheritance of any option from parent router.
    pub fn base_config() -> Self {
        MaterializedConfig::default().into()
    }

    /// Set to false if you want to disable autohandling of HEAD requests.
    pub fn handle_head<T: Into<Option<bool>>>(mut self, handle_head: T) -> Self {
        self.handle_head = handle_head.into();
        self
    }

    /// Additional headers that should be added to every response.
    /// NOTE: The headers are not going to override headers that are already set!
    pub fn extra_headers<T: Into<Option<Headers>>>(mut self, extra_headers: T) -> Self {
        let headers = extra_headers.into();
        self.extra_headers = headers.map(|headers| {
            let mut extra_headers = Vec::new();
            for header in headers.iter() {
                extra_headers.push((
                        header.name().to_owned(),
                        header.raw().iter().map(|x| x.to_vec()).collect(),
                        ));
            }
            extra_headers
        });
        self
    }

    /// Use other config settings for unset options.
    pub fn add(&mut self, other: &Config) {
        let other = other.to_owned();
        self.handle_head = self.handle_head.or(other.handle_head);
        self.extra_headers = self.extra_headers.take().or(other.extra_headers);
    }

    /// Convert this config into materialized config.
    pub(crate) fn materialize(&self) -> MaterializedConfig {
        let base = MaterializedConfig::default();

        MaterializedConfig {
            handle_head: self.handle_head.clone().unwrap_or(base.handle_head),
            extra_headers: self.extra_headers.clone().unwrap_or(base.extra_headers),
        }
    }
}

#[derive(Debug)]
pub(crate) struct MaterializedConfig {
    pub handle_head: bool,
    pub extra_headers: InternalHeaders,
}

impl From<MaterializedConfig> for Config {
    fn from(conf: MaterializedConfig) -> Self {
        Config {
            handle_head: Some(conf.handle_head),
            extra_headers: Some(conf.extra_headers),
        }
    }
}

impl Default for MaterializedConfig {
    fn default() -> Self {
        MaterializedConfig {
            handle_head: true,
            extra_headers: Default::default(),
        }
    }
}
