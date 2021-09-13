use clap::{App, Arg};
use fonttools::font;
use otspec::types::Offset16;
use otspec::types::OffsetMarkerTrait;
use petgraph::dot::Dot;
use petgraph::graph::{Graph, NodeIndex};
use std::fs::File;

fn main() {
    let matches = App::new("ttf-dump-graph")
        .about("Dumps an object graph")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true),
        )
        .arg(
            Arg::with_name("TABLE")
                .help("Sets the table to dump")
                .required(true),
        )
        .get_matches();
    let filename = matches.value_of("INPUT").unwrap();
    let infile = File::open(filename).unwrap();

    let mut infont = font::load(infile).unwrap();
    let table_name = matches.value_of("TABLE").unwrap();
    let root = if table_name == "GSUB" {
        let gsub = infont
            .get_table(b"GSUB")
            .expect("Error reading GSUB table")
            .expect("No GSUB table found")
            .gsub_unchecked();
        Offset16::to(gsub)
    } else {
        let gpos = infont
            .get_table(b"GPOS")
            .expect("Error reading GPOS table")
            .expect("No GPOS table found")
            .gsub_unchecked();
        Offset16::to(gpos)
    };

    let mut dag: Graph<&dyn OffsetMarkerTrait, Option<usize>> = Graph::new();
    add_object_graph(&mut dag, &root);
    println!("{:#?}", Dot::new(&dag));
}

fn add_object_graph<'a>(
    dag: &mut Graph<&'a dyn OffsetMarkerTrait, Option<usize>>,
    obj: &'a dyn OffsetMarkerTrait,
) -> NodeIndex<u32> {
    let mut children = vec![];
    for f in obj.children() {
        children.push(add_object_graph(dag, &*f));
    }
    let node = dag.add_node(obj);
    for child in children {
        let mut hack: Vec<u8> = vec![];
        obj.serialize_offset(&mut hack).expect("No wait");
        let off: u16 = otspec::de::from_bytes(&hack).unwrap();
        dag.add_edge(node, child, Some(off.into()));
    }
    node
}
