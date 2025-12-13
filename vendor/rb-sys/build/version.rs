use crate::RbConfig;

#[allow(dead_code)]
pub const LATEST_STABLE_VERSION: Version = Version::new(3, 4);
#[allow(dead_code)]
pub const MIN_SUPPORTED_STABLE_VERSION: Version = Version::new(2, 6);

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Version(u32, u32);

impl Version {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self(major, minor)
    }

    pub fn major(&self) -> u32 {
        self.0
    }

    pub fn minor(&self) -> u32 {
        self.1
    }

    pub fn current(rbconfig: &RbConfig) -> Version {
        match (rbconfig.get("MAJOR"), rbconfig.get("MINOR")) {
            (Some(major), Some(minor)) => {
                Version::new(major.parse::<u32>().unwrap(), minor.parse::<u32>().unwrap())
            }
            _ => {
                // Try to parse out the first 3 components of the version string (for truffleruby)
                let version_string = rbconfig.get("ruby_version").expect("ruby_version");
                let mut parts = version_string.split('.').map(|s| s.parse::<u32>());
                let major = parts.next().expect("major").unwrap();
                let minor = parts.next().expect("minor").unwrap();
                Version::new(major, minor)
            }
        }
    }

    #[allow(dead_code)]
    pub fn is_stable(&self) -> bool {
        *self >= MIN_SUPPORTED_STABLE_VERSION && *self <= LATEST_STABLE_VERSION
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}
