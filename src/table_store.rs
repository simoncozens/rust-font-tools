use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    fmt::Debug,
    ops::Deref,
    rc::Rc,
};

use otspec::types::Tag;
use otspec::{DeserializationError, ReaderContext, SerializationError, Serialize};

use crate::tables;

/// A helper used to build a `TableSet` during deserialization.
///
/// This ensures that a newly constructed table preloads required tables,
/// and ensures that the TableSet is initialized correctly.
#[derive(Debug, Default)]
pub struct TableLoader {
    inner: TableSet,
}

/// A lazy loader for the set of OpenType tables in a font.
#[derive(Debug, Default)]
pub struct TableSet {
    tables: BTreeMap<Tag, RefCell<LazyItem>>,
}

/// A table in a font, which may or may not have been loaded yet.
#[derive(Debug, PartialEq)]
enum LazyItem {
    Unloaded(Rc<[u8]>),
    Loaded(Table),
}

/// Any OpenType table.
//TODO: we can probably get rid of this and just use `TableData`?
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    tag: Tag,
    data: TableData,
}

impl Table {
    /// If this table is of a known type, return a `KnownTable`.
    pub fn known(&self) -> Option<KnownTable> {
        match &self.data {
            TableData::Known { typed, .. } => Some(typed.clone()),
            _ => None,
        }
    }

    /// If this table is of an unknown type, return the raw bytes.
    pub fn unknown(&self) -> Option<Rc<[u8]>> {
        match &self.data {
            TableData::Unknown { raw } => Some(raw.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
enum TableData {
    /// A table of a known type.
    Known {
        /// The raw data for serialization.
        ///
        /// This is only `Some` if this table was loaded from existing data,
        /// or if this table has already been serialized.
        raw: Option<Rc<[u8]>>,
        /// The typed table. This is one of the types defined in the `tables`
        /// module.
        typed: KnownTable,
    },
    /// A table we don't know how to represent; we just hold on to the bits.
    Unknown { raw: Rc<[u8]> },
}

/// An OpenType table of a known format.
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum KnownTable {
    /// Contains an axis variations table.
    avar(Rc<tables::avar::avar>),
    /// Contains a character to glyph index mapping table.
    cmap(Rc<tables::cmap::cmap>),
    /// Contains a font variations table.
    fvar(Rc<tables::fvar::fvar>),
    /// Contains a grid-fitting and scan-conversion procedure table.
    gasp(Rc<tables::gasp::gasp>),
    /// Contains a tables::glyph::glyph definition table.
    GDEF(Rc<tables::GDEF::GDEF>),
    /// Contains a glyph positioning table.
    GPOS(Rc<tables::GPOS::GPOS>),
    /// Contains a glyph substitution table.
    GSUB(Rc<tables::GSUB::GSUB>),
    /// Contains a glyph data table.
    glyf(Rc<tables::glyf::glyf>),
    /// Contains a glyph variations table.
    gvar(Rc<tables::gvar::gvar>),
    /// Contains a header table.
    head(Rc<tables::head::head>),
    /// Contains a horizontal header table.
    hhea(Rc<tables::hhea::hhea>),
    /// Contains a horizontal metrics table.
    hmtx(Rc<tables::hmtx::hmtx>),
    /// Contains an index-to-location table.
    loca(Rc<tables::loca::loca>),
    /// Contains a math typesetting table.
    MATH(Rc<tables::MATH::MATH>),
    /// Contains a maximum profile table.
    maxp(Rc<tables::maxp::maxp>),
    /// Contains a naming table.
    name(Rc<tables::name::name>),
    /// Contains an OS/2 and Windows metrics table.
    os2(Rc<tables::os2::os2>),
    /// Contains a postscript table.
    post(Rc<tables::post::post>),
    /// Contains a style attributes table.
    STAT(Rc<tables::STAT::STAT>),
}

/// A reference-counted pointer with transparent copy-on-write semantics.
///
/// Accessing a mutating method on the inner type will cause a clone of
/// the inner data if there are other outstanding references to this pointer.
///
/// # Note
///
/// This may be too fancy? It is an attempt to make the API more clear, by
/// avoiding the need to do `Rc::make_mut(&mut my_table)` every time the user
/// wants to mutate a table.
pub struct Cow<T> {
    inner: Rc<T>,
}

impl<T: Clone> std::ops::Deref for Cow<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// any access to a mutating method will cause us to ensure we point to unique
// data.
impl<T: Clone> std::ops::DerefMut for Cow<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Rc::make_mut(&mut self.inner)
    }
}

impl TableLoader {
    /// Add a raw table to the `TableSet`.
    pub fn add(&mut self, tag: Tag, data: Rc<[u8]>) {
        self.inner
            .tables
            .insert(tag, RefCell::new(LazyItem::Unloaded(data)));
    }

    /// Load required tables and return the `TableSet`.
    pub fn finish(self) -> Result<TableSet, DeserializationError> {
        let tables = self.inner;
        tables.load_if_needed(tables::head::TAG)?;
        tables.load_if_needed(tables::loca::TAG)?;
        Ok(tables)
    }
}

impl TableSet {
    /// The number of tables in this set.
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    /// Attempt to deserialize all the tables in this set.
    pub fn fully_deserialize(&self) -> Result<(), DeserializationError> {
        // Order is important
        self.load_if_needed(tables::head::TAG)?;
        self.load_if_needed(tables::maxp::TAG)?;
        if self.tables.contains_key(&tables::glyf::TAG) {
            self.load_if_needed(tables::loca::TAG)?;
            self.load_if_needed(tables::glyf::TAG)?;
        }

        for tag in self.keys() {
            if let Err(e) = self.load_if_needed(tag) {
                log::warn!("Couldn't deserilaize {}: '{}'", tag, e);
            }
        }

        Ok(())
    }

    /// Get a table, attempting to load it if necessary.
    ///
    /// For known tables, you should prefer to use the typed methods such
    /// as [`name`], [`GDEF`], etc. These methods exist for each table defined
    /// in the [`tables`] module.
    ///
    /// [`name`]: TableSet::name
    /// [`GDEF`]: TableSet::GDEF
    /// [`tables`]: crate::tables
    pub fn get(&self, tag: Tag) -> Result<Option<Table>, DeserializationError> {
        self.load_if_needed(tag)?;
        Ok(self
            .tables
            .get(&tag)
            .map(|item| item.borrow().loaded().unwrap()))
    }

    /// Returns an iterator over the `Tag`s of the tables in this set.
    pub fn keys(&self) -> impl Iterator<Item = Tag> + '_ {
        self.tables.keys().cloned()
    }

    /// Returns `true` if the provided tag is a table in this `TableSet`.
    pub fn contains<Q>(&self, table: &Q) -> bool
    where
        Tag: Borrow<Q>,
        Q: Ord,
    {
        self.tables.contains_key(table)
    }

    /// Remove a table from this set.
    ///
    /// If the exists and was loaded, it is returned.
    //TODO: is this too fancy? we don't want to return `LazyItem`
    // (which is private API) and we also don't want to try loading before
    // removing. Will anyone use this? one *possible* case is slightly more
    // efficient mutation, avoiding the table clone?
    pub fn remove(&mut self, table: Tag) -> Option<Table> {
        self.tables
            .remove(&table)
            .and_then(|item| item.into_inner().loaded())
    }

    /// Insert a known table into this set, replacing any existing table.
    ///
    /// To insert an unknown table, use [`insert_raw`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn make_name_table() -> tables::name::name { unreachable!("just for show") }
    /// use fonttools::{tables, font::{SfntVersion, Font}};
    ///
    /// let mut font = Font::new(SfntVersion::OpenType);
    /// let name_table: tables::name::name = make_name_table();
    /// font.tables.insert(name_table);
    /// ```
    pub fn insert(&mut self, table: impl Into<Table>) {
        let table = table.into();
        let tag = table.tag;
        let table = RefCell::new(LazyItem::Loaded(table));
        self.tables.insert(tag, table);
    }

    /// Assign raw binary data to a table.
    ///
    /// If the table is typed and exists, it is removed and will be reloaded
    /// on the next access.
    fn insert_raw(&mut self, tag: Tag, data: impl Into<Rc<[u8]>>) {
        self.tables
            .insert(tag, RefCell::new(LazyItem::Unloaded(data.into())));
    }

    fn load_if_needed(&self, tag: Tag) -> Result<(), DeserializationError> {
        let item = match self.tables.get(&tag) {
            Some(item) => item,
            _ => return Ok(()),
        };

        let to_load = match &*item.borrow() {
            LazyItem::Unloaded(data) => data.clone(),
            LazyItem::Loaded(_) => return Ok(()),
        };

        let loaded_table = self.deserialize_table(tag, to_load)?;
        *item.borrow_mut() = LazyItem::Loaded(loaded_table);
        Ok(())
    }

    fn deserialize_table(&self, tag: Tag, data: Rc<[u8]>) -> Result<Table, DeserializationError> {
        let typed_data: Option<KnownTable> = match tag.as_bytes() {
            b"avar" => Some(otspec::de::from_bytes::<tables::avar::avar>(&data)?.into()),
            b"cmap" => Some(otspec::de::from_bytes::<tables::cmap::cmap>(&data)?.into()),
            b"fvar" => Some(otspec::de::from_bytes::<tables::fvar::fvar>(&data)?.into()),
            b"gasp" => Some(otspec::de::from_bytes::<tables::gasp::gasp>(&data)?.into()),
            b"GDEF" => Some(otspec::de::from_bytes::<tables::GDEF::GDEF>(&data)?.into()),
            b"GPOS" => Some(otspec::de::from_bytes::<tables::GPOS::GPOS>(&data)?.into()),
            b"GSUB" => Some(otspec::de::from_bytes::<tables::GSUB::GSUB>(&data)?.into()),
            b"head" => Some(otspec::de::from_bytes::<tables::head::head>(&data)?.into()),
            b"hhea" => Some(otspec::de::from_bytes::<tables::hhea::hhea>(&data)?.into()),
            b"MATH" => Some(otspec::de::from_bytes::<tables::MATH::MATH>(&data)?.into()),
            b"maxp" => Some(otspec::de::from_bytes::<tables::maxp::maxp>(&data)?.into()),
            b"name" => Some(otspec::de::from_bytes::<tables::name::name>(&data)?.into()),
            b"OS/2" => Some(otspec::de::from_bytes::<tables::os2::os2>(&data)?.into()),
            b"post" => Some(otspec::de::from_bytes::<tables::post::post>(&data)?.into()),
            b"STAT" => Some(otspec::de::from_bytes::<tables::STAT::STAT>(&data)?.into()),
            b"hmtx" => {
                let number_of_hmetrics = self
                    //TODO: dear reviewer: this loads the table if missing. do
                    //we want to force the user to do this explicitly?
                    .hhea()?
                    .map(|hhea| hhea.numberOfHMetrics)
                    //TODO: are we allowed to not have hhea?
                    .ok_or_else(|| DeserializationError("deserialize hhea before hmtx".into()))?;
                Some(
                    tables::hmtx::from_bytes(
                        &mut ReaderContext::new(data.to_vec()),
                        number_of_hmetrics,
                    )?
                    .into(),
                )
            }
            b"loca" => {
                let is_32bit = self
                    .head()?
                    .map(|head| head.indexToLocFormat == 1)
                    .ok_or_else(|| DeserializationError("deserialize head before loca".into()))?;
                Some(
                    tables::loca::from_bytes(&mut ReaderContext::new(data.to_vec()), is_32bit)?
                        .into(),
                )
            }
            b"glyf" => {
                let loca = self
                    .loca()?
                    .ok_or_else(|| DeserializationError("deserialize loca before glyf".into()))?;
                Some(tables::glyf::from_bytes(&data, &loca.indices)?.into())
            }
            b"gvar" => {
                let glyf = self
                    .glyf()?
                    .ok_or_else(|| DeserializationError("deserialize glyf before gvar".into()))?;
                let coords_and_ends = glyf
                    .glyphs
                    .iter()
                    .map(|g| g.gvar_coords_and_ends())
                    .collect();

                Some(tables::gvar::from_bytes(&data, coords_and_ends)?.into())
            }
            _ => None,
        };

        let data = match typed_data {
            Some(typed) => TableData::Known {
                raw: Some(data),
                typed,
            },
            None => TableData::Unknown { raw: data },
        };
        Ok(Table { data, tag })
    }

    fn is_serialized(&self, tag: Tag) -> Option<bool> {
        self.tables
            .get(&tag)
            .map(|table| match table.borrow().deref() {
                LazyItem::Unloaded(_) => true,
                LazyItem::Loaded(table) => match &table.data {
                    TableData::Known { raw, .. } => raw.is_some(),
                    TableData::Unknown { .. } => true,
                },
            })
    }

    pub(crate) fn compile_glyf_loca_maxp(&mut self) {
        // leave early if we have no work to do.
        if self.is_serialized(tables::glyf::TAG).unwrap_or(true)
            && self.is_serialized(tables::loca::TAG).unwrap_or(true)
            && self.is_serialized(tables::maxp::TAG).unwrap_or(true)
        {
            return;
        }
        let glyf = match self.glyf().unwrap() {
            Some(table) => table,
            None => {
                println!("Warning: no glyf table");
                return;
            }
        };
        let glyf_count = glyf.glyphs.len();
        let mut glyf_output: Vec<u8> = vec![];
        let mut loca_indices: Vec<u32> = vec![];

        for g in &glyf.glyphs {
            let cur_len: u32 = glyf_output.len().try_into().unwrap();
            loca_indices.push(cur_len);
            if g.is_empty() {
                continue;
            }
            glyf_output.extend(otspec::ser::to_bytes(&g).unwrap());
            // Add multiple-of-four padding
            while glyf_output.len() % 4 != 0 {
                glyf_output.push(0);
            }
        }
        loca_indices.push(glyf_output.len().try_into().unwrap());
        let loca_is32bit = u16::try_from(glyf_output.len()).is_err();

        let loca_data = if loca_is32bit {
            otspec::ser::to_bytes(&loca_indices).unwrap()
        } else {
            let mut data = Vec::with_capacity(loca_indices.len() * 2);
            loca_indices
                .iter()
                .map(|x| ((*x / 2) as u16))
                .for_each(|x| x.to_bytes(&mut data).unwrap());
            data
        };

        self.insert_raw(tables::glyf::TAG, glyf_output);
        self.insert_raw(tables::loca::TAG, loca_data);

        let mut maxp = self.maxp().unwrap().unwrap().clone();
        maxp.set_num_glyphs(glyf_count.try_into().unwrap());

        let mut head = self.head().unwrap().unwrap().clone();
        head.indexToLocFormat = if loca_is32bit { 1 } else { 0 };
        self.insert(head);

        if let Some(hmetric_count) = self.hmtx().unwrap().map(|t| t.number_of_hmetrics()) {
            if let Some(mut hhea) = self.hhea().unwrap() {
                hhea.numberOfHMetrics = hmetric_count;
                self.insert(hhea);
            }
        }
    }

    pub(crate) fn write_table(
        &self,
        tag: Tag,
        buffer: &mut Vec<u8>,
    ) -> Result<(), SerializationError> {
        let table = match self.tables.get(&tag) {
            Some(table) => table,
            None => return Ok(()),
        };

        match &*table.borrow() {
            LazyItem::Unloaded(raw) => raw.to_bytes(buffer),
            LazyItem::Loaded(table) => match &table.data {
                TableData::Unknown { raw } => raw.to_bytes(buffer),
                TableData::Known {
                    raw: Some(data), ..
                } => data.to_bytes(buffer),
                TableData::Known { typed, .. } => typed.to_bytes(buffer),
            },
        }
    }
}

impl LazyItem {
    fn loaded(&self) -> Option<Table> {
        match self {
            LazyItem::Unloaded(_) => None,
            LazyItem::Loaded(thing) => Some(thing.clone()),
        }
    }
}

impl PartialEq for TableSet {
    fn eq(&self, other: &Self) -> bool {
        self.tables.len() == other.tables.len()
            && self
                .tables
                .iter()
                .zip(other.tables.iter())
                .all(|((k1, v1), (k2, v2))| k1 == k2 && *v1.borrow() == *v2.borrow())
    }
}

impl PartialEq for TableData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Known { typed: one, .. }, Self::Known { typed: two, .. }) => one == two,
            (Self::Unknown { raw: one }, Self::Unknown { raw: two }) => one == two,
            _ => false,
        }
    }
}

impl KnownTable {
    fn tag(&self) -> Tag {
        match self {
            KnownTable::avar(_) => tables::avar::TAG,
            KnownTable::cmap(_) => tables::cmap::TAG,
            KnownTable::fvar(_) => tables::fvar::TAG,
            KnownTable::gasp(_) => tables::gasp::TAG,
            KnownTable::GDEF(_) => tables::GDEF::TAG,
            KnownTable::GPOS(_) => tables::GPOS::TAG,
            KnownTable::GSUB(_) => tables::GSUB::TAG,
            KnownTable::glyf(_) => tables::glyf::TAG,
            KnownTable::gvar(_) => tables::gvar::TAG,
            KnownTable::head(_) => tables::head::TAG,
            KnownTable::hhea(_) => tables::hhea::TAG,
            KnownTable::hmtx(_) => tables::hmtx::TAG,
            KnownTable::loca(_) => tables::loca::TAG,
            KnownTable::maxp(_) => tables::maxp::TAG,
            KnownTable::name(_) => tables::name::TAG,
            KnownTable::os2(_) => tables::os2::TAG,
            KnownTable::post(_) => tables::post::TAG,
            KnownTable::STAT(_) => tables::STAT::TAG,
            KnownTable::MATH(_) => tables::MATH::TAG,
        }
    }
}

// a little helper to ensure at compile time that tables are Clone;
// we need this for copy-on-write semanatics
fn assert_is_clone<T: Clone>() {}

macro_rules! from_table {
    ($table:ty, $enum: ident) => {
        impl From<$table> for KnownTable {
            fn from(src: $table) -> KnownTable {
                KnownTable::$enum(Rc::new(src))
            }
        }

        impl From<$table> for Table {
            fn from(src: $table) -> Table {
                Table::from(Cow {
                    inner: Rc::new(src),
                })
            }
        }

        impl From<Cow<$table>> for Table {
            fn from(src: Cow<$table>) -> Table {
                let typed = KnownTable::$enum(src.inner);
                let tag = typed.tag();
                Table {
                    data: TableData::Known { raw: None, typed },
                    tag,
                }
            }
        }

        impl KnownTable {
            #[allow(non_snake_case)]
            fn $enum(&self) -> Cow<$table> {
                assert_is_clone::<$table>();

                if let Self::$enum(table) = self {
                    return Cow {
                        inner: table.clone(),
                    };
                } else {
                    panic!("expected '{}' found '{}'", stringify!($ident), self.tag());
                }
            }
        }

        impl TableSet {
            #[allow(non_snake_case)]

            pub fn $enum(&self) -> Result<Option<Cow<$table>>, DeserializationError> {
                Ok(self.get(tables::$enum::TAG)?.map(|t| t.known().expect(stringify!($enum is known table type)).$enum()))
            }
        }
    };
}

from_table!(tables::GDEF::GDEF, GDEF);
from_table!(tables::GPOS::GPOS, GPOS);
from_table!(tables::GSUB::GSUB, GSUB);
from_table!(tables::STAT::STAT, STAT);
from_table!(tables::avar::avar, avar);
from_table!(tables::cmap::cmap, cmap);
from_table!(tables::fvar::fvar, fvar);
from_table!(tables::gasp::gasp, gasp);
from_table!(tables::glyf::glyf, glyf);
from_table!(tables::gvar::gvar, gvar);
from_table!(tables::head::head, head);
from_table!(tables::hhea::hhea, hhea);
from_table!(tables::hmtx::hmtx, hmtx);
from_table!(tables::loca::loca, loca);
from_table!(tables::maxp::maxp, maxp);
from_table!(tables::name::name, name);
from_table!(tables::os2::os2, os2);
from_table!(tables::post::post, post);
from_table!(tables::MATH::MATH, MATH);

impl Serialize for KnownTable {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        match self {
            KnownTable::avar(expr) => expr.to_bytes(data),
            KnownTable::cmap(expr) => expr.to_bytes(data),
            KnownTable::fvar(expr) => expr.to_bytes(data),
            KnownTable::gasp(expr) => expr.to_bytes(data),
            KnownTable::GSUB(expr) => expr.to_bytes(data),
            KnownTable::GDEF(expr) => expr.to_bytes(data),
            KnownTable::GPOS(expr) => expr.to_bytes(data),
            KnownTable::gvar(_) => unimplemented!(),
            KnownTable::head(expr) => expr.to_bytes(data),
            KnownTable::hhea(expr) => expr.to_bytes(data),
            KnownTable::hmtx(_) => unimplemented!(),
            KnownTable::glyf(_) => unimplemented!(),
            KnownTable::loca(_) => unimplemented!(),
            KnownTable::maxp(expr) => expr.to_bytes(data),
            KnownTable::MATH(_) => unimplemented!(),
            KnownTable::name(expr) => expr.to_bytes(data),
            KnownTable::os2(expr) => expr.to_bytes(data),
            KnownTable::post(expr) => expr.to_bytes(data),
            KnownTable::STAT(expr) => expr.to_bytes(data),
        }
    }
}
