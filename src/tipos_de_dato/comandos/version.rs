pub struct Version;

impl Version {
    pub fn from(_args: Vec<String>) -> Result<Version, String> {
        Ok(Version)
    }

    pub fn ejecutar(&self) -> Result<String, String> {
        Ok("gir version 2.0".to_string())
    }
}
