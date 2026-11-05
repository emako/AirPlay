#[derive(Clone, Debug)]
pub struct Track {
    pub location: String,
    pub title: String,
    pub start: f64,
    pub end: Option<f64>,
    pub is_group: bool,
    pub missing: bool,
}

impl Track {
    pub fn playable(&self) -> bool {
        !self.is_group && !self.location.is_empty()
    }

    pub fn group(name: impl Into<String>) -> Self {
        Self {
            location: String::new(),
            title: name.into(),
            start: 0.0,
            end: None,
            is_group: true,
            missing: false,
        }
    }
}
