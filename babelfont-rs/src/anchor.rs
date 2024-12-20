#[derive(Debug, Clone)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
    pub name: String,
}

#[cfg(feature = "ufo")]
impl From<&norad::Anchor> for Anchor {
    fn from(a: &norad::Anchor) -> Self {
        Anchor {
            x: a.x as f32,
            y: a.y as f32,
            name: a
                .name
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or_else(|| "<Unnamed anchor>".to_string()),
        }
    }
}
