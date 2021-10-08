use std::borrow::Borrow;
use std::{any::Any, cell::RefCell, collections::BTreeMap, rc::Rc};

use otspec::types::{tag, Tag};
use otspec::{DeserializationError, ReaderContext};

use crate::tables;

/// A lazy loader for the set of OpenType tables in a font.
#[derive(Debug, Default)]
pub struct TableSet {
    tables: BTreeMap<Tag, RefCell<LazyItem>>,
}

#[derive(Debug)]
enum LazyItem {
    Unloaded(Rc<[u8]>),
    Loaded(Rc<dyn Any>),
    Error(DeserializationError),
}

/// An unknown table.
#[derive(Debug, Clone)]
pub struct UnknownTable {
    /// The raw data for this table.
    pub data: Rc<[u8]>,
}

impl TableSet {
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    /// Get a table that has already been loaded.
    ///
    /// If the table has not been loaded this will return `None`, even if it
    /// exists.
    pub fn get_no_load<T: Any>(&self, tag: Tag) -> Option<&T> {
        self.tables
            .get(&tag)
            .and_then(|item| item.borrow().loaded())
            .map(|t| t.downcast_ref().expect("invalid type in table"))
    }

    /// Get a table, attempting to load it if necessary.
    pub fn get<T: Any>(&self, tag: Tag) -> Result<Option<&T>, DeserializationError> {
        self.load_if_needed(tag)?;
        Ok(self.get_no_load(tag))
    }

    /// Returns `true` if the provided tag is a table in this `TableSet`.
    pub fn contains<Q>(&self, table: &Q) -> bool
    where
        Tag: Borrow<Q>,
        Q: Ord,
    {
        self.tables.contains_key(table)
    }

    pub fn remove(&mut self, table: Tag) -> Option<()> {
        self.tables.remove(&table).map(|_| ())
    }

    fn load_if_needed(&self, tag: Tag) -> Result<(), DeserializationError> {
        let item = match self.tables.get(&tag) {
            Some(item) => item,
            _ => return Ok(()),
        };

        let to_load = match *item.borrow() {
            LazyItem::Unloaded(data) => data.clone(),
            LazyItem::Error(e) => return Err(e.clone()),
            LazyItem::Loaded(_) => unreachable!(),
        };

        let loaded_table = self.deserialize_table(tag, to_load)?;
        *item.borrow_mut() = LazyItem::Loaded(loaded_table);
        Ok(())
    }

    fn deserialize_table(
        &self,
        tag: Tag,
        data: Rc<[u8]>,
    ) -> Result<Rc<dyn Any>, DeserializationError> {
        let table: Rc<dyn Any> = match tag.as_bytes() {
            b"avar" => Rc::new(otspec::de::from_bytes::<tables::avar::avar>(&data)?),
            b"cmap" => Rc::new(otspec::de::from_bytes::<tables::cmap::cmap>(&data)?),
            b"fvar" => Rc::new(otspec::de::from_bytes::<tables::fvar::fvar>(&data)?),
            b"gasp" => Rc::new(otspec::de::from_bytes::<tables::gasp::gasp>(&data)?),
            b"GDEF" => Rc::new(otspec::de::from_bytes::<tables::GDEF::GDEF>(&data)?),
            b"GPOS" => Rc::new(otspec::de::from_bytes::<tables::GPOS::GPOS>(&data)?),
            b"GSUB" => Rc::new(otspec::de::from_bytes::<tables::GSUB::GSUB>(&data)?),
            b"head" => Rc::new(otspec::de::from_bytes::<tables::head::head>(&data)?),
            b"hhea" => Rc::new(otspec::de::from_bytes::<tables::hhea::hhea>(&data)?),
            b"MATH" => Rc::new(otspec::de::from_bytes::<tables::MATH::MATH>(&data)?),
            b"maxp" => Rc::new(otspec::de::from_bytes::<tables::maxp::maxp>(&data)?),
            b"name" => Rc::new(otspec::de::from_bytes::<tables::name::name>(&data)?),
            b"OS/2" => Rc::new(otspec::de::from_bytes::<tables::os2::os2>(&data)?),
            b"post" => Rc::new(otspec::de::from_bytes::<tables::post::post>(&data)?),
            b"STAT" => Rc::new(otspec::de::from_bytes::<tables::STAT::STAT>(&data)?),
            b"hmtx" => {
                let number_of_hmetrics = self
                    .get_no_load::<tables::hhea::hhea>(tag!("hhea"))
                    .map(|hhea| hhea.numberOfHMetrics)
                    .ok_or_else(|| DeserializationError("deserialize hhea before hmtx".into()))?;
                Rc::new(tables::hmtx::from_bytes(
                    &mut ReaderContext::new(data.to_vec()),
                    number_of_hmetrics,
                )?)
            }
            b"loca" => {
                let is_32bit = self
                    .get_no_load::<tables::head::head>(tag!("head"))
                    .map(|head| head.indexToLocFormat == 1)
                    .ok_or_else(|| DeserializationError("deserialize head before loca".into()))?;
                Rc::new(tables::loca::from_bytes(
                    &mut ReaderContext::new(data.to_vec()),
                    is_32bit,
                )?)
            }
            b"glyf" => {
                let loca_offsets = self
                    .get_no_load::<tables::loca::loca>(tag!("loca"))
                    .map(|loca| &loca.indices)
                    .ok_or_else(|| DeserializationError("deserialize loca before glyf".into()))?;
                Rc::new(tables::glyf::from_bytes(&data, loca_offsets)?)
            }
            b"gvar" => {
                let glyf = self
                    .get_no_load::<tables::glyf::glyf>(tag!("glyf"))
                    //.map(|glyf| &loca.indices)
                    .ok_or_else(|| DeserializationError("deserialize glyf before gvar".into()))?;
                let coords_and_ends = glyf
                    .glyphs
                    .iter()
                    .map(|g| g.gvar_coords_and_ends())
                    .collect();

                Rc::new(tables::gvar::from_bytes(&data, coords_and_ends)?)
            }
            _ => Rc::new(data.to_owned()),
        };
        Ok(table)
    }
}

impl LazyItem {
    fn loaded(&self) -> Option<&dyn Any> {
        match self {
            LazyItem::Unloaded(_) => None,
            LazyItem::Error(_) => None,
            LazyItem::Loaded(thing) => Some(thing),
        }
    }

    fn needs_load(&self) -> bool {
        matches!(self, LazyItem::Unloaded(_))
    }
}

//fn load(&mut self, tag: Tag) -> Result<(), DeserializationError> {
//let data = match self.unloaded.remove(&tag) {
//Some(data) => data,
//None => return Ok(()),
//};

//let data: Rc<dyn Any> = match tag.as_str() {
//"avar" => Rc::new(otspec::de::from_bytes::<tables::avar::avar>(&data)?),
//"cmap" => Rc::new(otspec::de::from_bytes::<tables::cmap::cmap>(&data)?),
//// other tables here
//other => Rc::new(data),
//};
//self.loaded.insert(tag, data);
//Ok(())
//}

//fn load_tables(&mut self, tables: &[Tag]) -> Result<(), DeserializationError> {
//for tag in tables {
//self.load(*tag)?;
//}
//Ok(())
//}

//fn needs_to_deserialize(&self, table: Tag) -> bool {
//self.unloaded.contains_key(&table)
//}

//fn get_or_deserialize<T: Any>(
//&mut self,
//tag: Tag,
//d: impl FnOnce(&[u8]) -> Result<T, DeserializationError>,
//) -> Result<Option<Rc<T>, DeserializationError>> {

//}

//fn gpos(&self) -> Option<&crate::GPOS::GPOS> {
//self.get_typed(GPOS_TAG)
//}

//fn gpos_mut(&mut self) -> Option<&mut crate::GPOS::GPOS> {
//self.get_typed_mut(GPOS_TAG)
//}

//fn get_typed<T: Any>(&self, tag: Tag) -> Option<&T> {
//assert!(
// !self.unloaded.borrow().contains_key(&tag),
//"tables must be loaded before use"
//);
//self.loaded
//.get(&tag)
//.map(|t| t.downcast_ref().expect("wrong type for tag"))
//}

//fn get_typed_mut<T: Any>(&mut self, tag: Tag) -> Option<&mut T> {
//assert!(
// !self.unloaded.contains_key(&tag),
//"tables must be loaded before use"
//);
//self.loaded
//.get_mut(&tag)
//.map(|t| t.downcast_mut().expect("wrong type for tag"))
//}

//fn other_table(&self, tag: Tag) -> Option<&[u8]> {
//self.unloaded.get(&tag).map(Vec::as_slice)
//}
//}

//#[derive(Debug, Default)]
//pub struct Tables(BTreeMap<Tag, RefCell<Table>>);

//impl Tables {
//pub fn len(&self) -> usize {
//self.0.len()
//}

//pub fn is_empty(&self) -> bool {
//self.0.is_empty()
//}

////pub fn iter(&self) -> impl Iterator<Item = (&Tag, &Table)> {
////self.0.iter().map(|(a, b)| (a, b.borrow().deref()))
////}

//pub fn keys(&self) -> impl Iterator<Item = &Tag> {
//self.0.keys()
//}

//pub fn contains_key(&self, key: Tag) -> bool {
//self.0.contains_key(&key)
//}

//pub fn get(&self, key: Tag) -> Option<&Table>
//{
//self.0.get(&key).map(RefCell::borrow).as_deref()
//}

//pub fn get_mut(&mut self, key: Tag) -> Option<&mut Table> {
//self.0.get(&key).map(RefCell::borrow_mut).as_deref_mut()
//}

//pub fn remove(&mut self, key: Tag) -> Option<Table>
//{
//self.0.remove(&key).map(RefCell::into_inner)
//}

//pub fn insert(&mut self, key: Tag, table: Table) -> Option<Table>
//{
//self.0.insert(key, RefCell::new(table)).map(RefCell::into_inner)
//}
//}
