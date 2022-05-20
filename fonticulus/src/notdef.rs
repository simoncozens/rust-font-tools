use babelfont::{Font, Glyph, GlyphCategory, Layer, Node, NodeType, Path, PathDirection, Shape};
use otspec::types::ot_round;

fn make_box<T>(x_min: T, y_min: T, x_max: T, y_max: T, reverse: bool) -> Vec<Node>
where
    T: Into<f32> + Copy,
{
    let mut v = vec![
        Node {
            x: x_min.into(),
            y: y_min.into(),
            nodetype: NodeType::Move,
        },
        Node {
            x: x_max.into(),
            y: y_min.into(),
            nodetype: NodeType::Line,
        },
        Node {
            x: x_max.into(),
            y: y_max.into(),
            nodetype: NodeType::Line,
        },
        Node {
            x: x_min.into(),
            y: y_max.into(),
            nodetype: NodeType::Line,
        },
        Node {
            x: x_min.into(),
            y: y_min.into(),
            nodetype: NodeType::Line,
        },
    ];
    if reverse {
        v.reverse();
        v[0].nodetype = NodeType::Move;
        v[4].nodetype = NodeType::Line;
    }
    v
}

pub(crate) fn add_notdef(input: &mut Font) {
    if input.glyphs.get(".notdef").is_some() {
        return;
    }
    let width = ot_round(input.upm as f32 * 0.5) as f32;
    let stroke: f32 = ot_round(input.upm as f32 * 0.05) as f32;

    let mut g = Glyph {
        name: ".notdef".to_string(),
        production_name: Some(".notdef".to_string()),
        category: GlyphCategory::Base,
        codepoints: vec![],
        layers: vec![],
        exported: true,
        direction: None,
    };
    for master in &input.masters {
        let ascender = master
            .metrics
            .get("ascender")
            .copied()
            .unwrap_or((input.upm as f32 * 0.80) as i32);
        let descender = master
            .metrics
            .get("descender")
            .copied()
            .unwrap_or(-(input.upm as f32 * 0.20) as i32);
        let x_min: f32 = stroke as f32;
        let x_max: f32 = (width - stroke) as f32;
        let y_max: f32 = ascender as f32;
        let y_min: f32 = descender as f32;
        let p1 = make_box(x_min, y_min, x_max, y_max, false);
        let p2 = make_box(
            x_min + stroke,
            y_min + stroke,
            x_max - stroke,
            y_max - stroke,
            true,
        );

        let mut l = Layer {
            width: width as i32,
            name: None,
            id: Some(master.id.clone()),
            guides: vec![],
            shapes: vec![],
            anchors: vec![],
            color: None,
            layer_index: None,
            is_background: false,
            background_layer_id: None,
            location: None,
        };
        l.shapes.push(Shape::PathShape(Path {
            nodes: p1,
            closed: true,
            direction: PathDirection::Clockwise,
        }));
        l.shapes.push(Shape::PathShape(Path {
            nodes: p2,
            closed: true,
            direction: PathDirection::Anticlockwise,
        }));
        g.layers.push(l);
    }
    input.glyphs.insert(0, g);
}
