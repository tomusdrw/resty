
/// A router configuration.
#[derive(Debug, Default, Clone)]
pub struct Config {
    handle_head: Option<bool>,
}

impl Config {
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

    /// Use other config settings for unset options.
    pub fn add(&mut self, other: &Config) {
        let other = other.to_owned();
        self.handle_head = self.handle_head.or(other.handle_head);
    }

    /// Convert this config into materialized config.
    pub(crate) fn materialize(&self) -> MaterializedConfig {
        let base = MaterializedConfig::default();

        MaterializedConfig {
            handle_head: self.handle_head.clone().unwrap_or(base.handle_head),
        }
    }
}

#[derive(Debug)]
pub(crate) struct MaterializedConfig {
    pub handle_head: bool,
}

impl From<MaterializedConfig> for Config {
    fn from(conf: MaterializedConfig) -> Self {
        Config {
            handle_head: Some(conf.handle_head),
        }
    }
}

impl Default for MaterializedConfig {
    fn default() -> Self {
        MaterializedConfig {
            handle_head: true,
        }
    }
}
