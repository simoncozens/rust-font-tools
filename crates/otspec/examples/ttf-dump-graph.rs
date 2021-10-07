use clap::{App, Arg};
use fonttools::font::{Font, Table};
use fonttools::tag;
use fonttools::MATH::MATHinternal;
use otspec::types::{Offset16, OffsetMarkerTrait};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{Graph, NodeIndex};

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

    let mut infont = Font::load(filename).unwrap();
    let table_name = matches.value_of("TABLE").unwrap();

    let mut dag: Graph<String, ()> = Graph::new();

    let graph = if table_name == "GSUB" {
        let gsub = infont
            .get_table(tag!("GSUB"))
            .expect("Error reading GSUB table")
            .expect("No GSUB table found")
            .gsub_unchecked();
        let gsub_off: &dyn OffsetMarkerTrait = &Offset16::to(gsub);
        add_object_graph(&mut dag, gsub_off);
        format!("{:?}", Dot::with_config(&dag, &[Config::EdgeNoLabel]))
    } else if table_name == "MATH" {
        // Get this at the lowest level
        let math_u8 = infont.tables.get(b"MATH").expect("No MATH table present");
        let math_internal: MATHinternal = if let Table::Unknown(binary) = math_u8 {
            otspec::de::from_bytes(binary).expect("Could not deserialize")
        } else {
            panic!("Something went wrong")
        };
        let math_internal_off: &dyn OffsetMarkerTrait = &Offset16::to(math_internal);
        add_object_graph(&mut dag, math_internal_off);
        format!("{:?}", Dot::with_config(&dag, &[Config::EdgeNoLabel]))
    } else {
        let gpos = infont
            .get_table(tag!("GPOS"))
            .expect("Error reading GPOS table")
            .expect("No GPOS table found")
            .gpos_unchecked();
        let gpos_off: &dyn OffsetMarkerTrait = &Offset16::to(gpos);
        add_object_graph(&mut dag, gpos_off);
        format!("{:?}", Dot::with_config(&dag, &[Config::EdgeNoLabel]))
    };
    println!("{}", graph.replace("\\\\n", "\\l"));
}

fn add_object_graph<'a>(
    dag: &mut Graph<String, ()>,
    obj: &'a dyn OffsetMarkerTrait,
) -> NodeIndex<u32> {
    let mut children = vec![];
    for f in obj.children() {
        children.push(add_object_graph(dag, &*f));
    }
    let node = dag.add_node(format!("{:#?}", obj));
    for child in children {
        dag.add_edge(node, child, ());
    }
    node
}
