use crate::types::uint16;
use crate::types::OffsetMarkerTrait;
use crate::Offset16;
use crate::SerializationError;
use crate::Serialize;

use petgraph::dot::Dot;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::visit::Topo;

pub struct OffsetManager<'a> {
    dag: Graph<&'a dyn OffsetMarkerTrait, Option<usize>>,
    resolved: bool,
}

impl<'a> OffsetManager<'a> {
    pub fn new<T: 'a>(obj: &'a T) -> Self
    where
        T: OffsetMarkerTrait,
    {
        let mut mgr = OffsetManager {
            dag: Graph::new(),
            resolved: false,
        };
        mgr.add_object_graph(obj);
        mgr
    }
    fn add_object_graph(&mut self, obj: &'a dyn OffsetMarkerTrait) -> NodeIndex<u32> {
        let mut children = vec![];
        for f in obj.children() {
            children.push(self.add_object_graph(&*f));
        }
        let node = self.dag.add_node(obj);
        for child in children {
            self.dag.add_edge(node, child, None);
        }
        node
    }
    pub fn dump_graph(&self) {
        println!("{:?}", Dot::new(&self.dag));
    }

    pub fn resolve(&mut self) {
        // First pass over the graph works out where everything's going to go.
        let mut topo = Topo::new(&self.dag);
        let mut node = topo.next(&self.dag);
        let mut offset_counter = 0;

        while node.is_some() {
            let this_offset = self.dag.node_weight(node.unwrap()).unwrap();
            // Set the edge to this
            let size = this_offset.object_size();
            if offset_counter > 0 {
                let parent = self
                    .dag
                    .edges_directed(node.unwrap(), petgraph::Direction::Incoming)
                    .next()
                    .unwrap()
                    .source();
                self.dag
                    .update_edge(parent, node.unwrap(), Some(offset_counter));
            }
            offset_counter += size; // Pad to multiple of 4 or whatever
            node = topo.next(&self.dag);
        }

        // Second pass over the graph works out the offsets, resetting to zero
        // at the top of each subtable.
        let mut topo = Topo::new(&self.dag);
        let mut node = topo.next(&self.dag);
        let mut base = 0;
        while node.is_some() {
            let this_offset = self.dag.node_weight(node.unwrap()).unwrap();
            let children_edges = self
                .dag
                .edges_directed(node.unwrap(), petgraph::Direction::Outgoing);
            let mut parent_edges = self
                .dag
                .edges_directed(node.unwrap(), petgraph::Direction::Incoming);
            if let Some(p) = parent_edges.next() {
                base = p.weight().unwrap();
            }

            for c in children_edges {
                let offset = if this_offset.is_top_of_table() {
                    c.weight().unwrap() - base
                } else {
                    c.weight().unwrap()
                };
                let target_id = c.target();
                let target_node = self.dag.node_weight(target_id).unwrap();
                target_node.set(offset as u16);
            }
            node = topo.next(&self.dag);
        }

        // self.dump_graph();
        self.resolved = true;
    }

    pub fn serialize(
        &mut self,
        output: &mut Vec<u8>,
        do_top: bool,
    ) -> Result<(), SerializationError> {
        assert!(self.resolved);
        let mut topo = Topo::new(&self.dag);
        let mut node = topo.next(&self.dag);
        if !do_top {
            if node.is_none() {
                return Ok(());
            }
            node = topo.next(&self.dag);
        }
        while node.is_some() {
            let this_offset = self.dag.node_weight(node.unwrap()).unwrap();
            this_offset.serialize_contents(output)?;
            node = topo.next(&self.dag);
        }
        Ok(())
    }
}

pub fn any_offsets_need_resolving<T>(obj: &T) -> bool
where
    T: Serialize,
{
    let fields = obj.offset_fields();
    if fields.is_empty() {
        return false;
    }
    for f in fields {
        if f.needs_resolving() {
            return true;
        }
    }
    false
}

pub fn resolve_offsets<T>(obj: T) -> T
where
    T: Serialize,
{
    let root = Offset16::to(obj);
    let mut mgr = OffsetManager::new(&root);
    mgr.resolve();
    root.link.unwrap()
}

pub fn resolve_offsets_and_serialize<T>(
    obj: T,
    output: &mut Vec<u8>,
    do_top: bool,
) -> Result<(), SerializationError>
where
    T: Serialize,
{
    let root = Offset16::to(obj);
    let mut mgr = OffsetManager::new(&root);
    mgr.resolve();
    mgr.serialize(output, do_top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as otspec;
    use crate::types::*;
    use otspec::Deserializer;
    use otspec_macros::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    struct One {
        #[serde(offset_base)]
        thing: uint16,
        anoffset: Offset16<Two>,
        other: uint16,
        asecondoffset: Offset16<Three>,
    }

    #[derive(Deserialize, Debug, PartialEq, Serialize, Clone)]
    struct Two {
        test1: uint16,
        deep: Offset16<Three>,
        test2: uint16,
    }

    #[derive(Deserialize, Debug, PartialEq, Serialize, Clone)]
    struct Three {
        blah: uint16,
    }

    #[test]
    fn test_resolve() {
        let one = One {
            thing: 0x01,
            anoffset: Offset16::to(Two {
                test1: 0x0a,
                deep: Offset16::to(Three { blah: 1010 }),
                test2: 0x0b,
            }),
            other: 2345,
            asecondoffset: Offset16::to(Three { blah: 2020 }),
        };
        assert_eq!(one.offset_fields().len(), 2);
        assert_eq!(one.anoffset.as_ref().unwrap().offset_fields().len(), 1);
        assert_eq!(
            one.anoffset
                .as_ref()
                .unwrap()
                .offset_fields()
                .first()
                .unwrap()
                .offset_fields()
                .len(),
            0
        );
        let one = resolve_offsets(one);
        assert_eq!(one.anoffset.offset_value(), Some(8));
        assert_eq!(one.asecondoffset.offset_value(), Some(16));

        let two = one.anoffset.as_ref().unwrap();
        assert_eq!(two.test1, 0x0a);
        assert_eq!(two.deep.offset_value(), Some(6));
    }

    #[test]
    fn test_serialize() {
        let one = One {
            thing: 0x01,
            anoffset: Offset16::to(Two {
                test1: 0x0a,
                deep: Offset16::to(Three { blah: 0x1010 }),
                test2: 0x0b,
            }),
            other: 0xaabb,
            asecondoffset: Offset16::to(Three { blah: 0x2020 }),
        };
        let mut output = vec![];
        resolve_offsets_and_serialize(one, &mut output, true).unwrap();
        assert_eq!(
            output,
            vec![
                0x0, 0x1, // thing = 0x1
                0x0, 0x8, // offset 8 to Two
                0xaa, 0xbb, // other = 0xaabb
                0x0, 0x10, // offset 16 to Three=0x2020
                // Two
                0x00, 0x0a, // test1
                0x00, 0x06, // offset 6 to Three = 0x1010
                0x00, 0x0b, // test2
                0x10, 0x10, // one.anoffset.deep = Three
                0x20, 0x20, // one.asecondoffset = Three
            ]
        );
    }

    #[test]
    fn test_serialize_magically() {
        let one = One {
            thing: 0x01,
            anoffset: Offset16::to(Two {
                test1: 0x0a,
                deep: Offset16::to(Three { blah: 0x1010 }),
                test2: 0x0b,
            }),
            other: 0xaabb,
            asecondoffset: Offset16::to(Three { blah: 0x2020 }),
        };
        let mut output = vec![];
        one.to_bytes(&mut output).unwrap();
        assert_eq!(
            output,
            vec![
                0x0, 0x1, // thing = 0x1
                0x0, 0x8, // offset 8 to Two
                0xaa, 0xbb, // other = 0xaabb
                0x0, 0x10, // offset 16 to Three=0x2020
                // Two
                0x00, 0x0a, // test1
                0x00, 0x06, // offset 6 to Three = 0x1010
                0x00, 0x0b, // one.anoffset.deep = Three
                0x10, 0x10, // one.asecondoffset = Three
                0x20, 0x20,
            ]
        );
    }
}
