#[derive(Debug)]
pub struct Anchor {
    pub x: i32,
    pub y: i32,
    pub name: String,
}

impl From<&norad::Anchor> for Anchor {
    fn from(a: &norad::Anchor) -> Self {
        Anchor {
            x: a.x as i32,
            y: a.y as i32,
            name: a
                .name
                .as_ref()
                .unwrap_or(&"<Unnamed anchor>".to_string())
                .to_string(),
        }
    }
}
