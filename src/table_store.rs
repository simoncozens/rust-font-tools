//! Lazy loading of font tables.
//!
//! The core of this module is the [`TableSet`], which represents the OpenType
//! tables in a font.
//!
//! [`TableSet`]: table_store::TableSet

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

use otspec::types::Tag;
use otspec::{DeserializationError, ReaderContext, SerializationError, Serialize};

use crate::tables;

/// A helper used to build a `TableSet` during deserialization.
///
/// This ensures that a newly constructed table preloads required tables,
/// and ensures that the TableSet is initialized correctly.
#[derive(Debug, Default)]
pub(crate) struct TableLoader {
    inner: TableSet,
}

/// A lazy loader for the set of OpenType tables in a font.
///
/// Tables are parsed into concrete types the first time they are accessed.
///
/// # Semantics
///
/// We use copy-on-write semantics for all tables. This means that when you
/// access a table, you receive a pointer to that table. The first time you
/// modify the table, we copy the data from the existing table to a new allocation.
///
/// If you modify a table and wish to have your modification reflected in the font,
/// you are responsible for inserting your newly modified copy of the table back
/// into the `TableSet`.
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
#[derive(Clone, Debug)]
pub struct Table {
    /// The table's tag. This is mostly used so that something implmenting
    /// `Into<Table>` knows its tag and can be inserted into the font.
    tag: Tag,
    /// The data from which this table was loaded. If the table is not mutated,
    /// we will write this out unchanged when we serialize.
    raw: Option<Rc<[u8]>>,
    loaded: LoadedTable,
}

/// A loaded OpenType table.
///
/// This represents all the known table types in their deserialized form.
/// It is mostly an implementation detail.
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum LoadedTable {
    /// Contains an axis variations table.
    avar(Rc<tables::avar::avar>),
    /// Contains a character to glyph index mapping table.
    cmap(Rc<tables::cmap::cmap>),
    /// Contains a control value table.
    cvt(Rc<tables::cvt::cvt>),
    /// Contains a font program table.
    fpgm(Rc<tables::fpgm::fpgm>),
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
    /// Contains a control value program table.
    prep(Rc<tables::prep::prep>),
    /// Contains a style attributes table.
    STAT(Rc<tables::STAT::STAT>),
    /// Any unknown table.
    Unknown(Rc<[u8]>),
}

/// A reference-counted pointer with copy-on-write semantics.
///
/// Accessing a mutating method on the inner type will cause a clone of
/// the inner data if there are other outstanding references to this pointer.
///
/// # Example
//FIXME: i'd like this to run but we need some convenience method
//for creating an actual font?
/// ```no_run
/// # fn load_font() -> Font { unimplemented!("just for show") }
/// use fonttools::{font::Font};
/// let mut font = load_font();
/// let mut head = font.tables.head().expect("failed to load").expect("missing head");
/// // the head table we've taken points to the same memory as the one in the font.
/// assert!(font.tables.head().unwrap().unwrap().ptr_eq(&head));
/// head.fontRevision = 1.1;
/// // after mutation, it no longer points to the same memory
/// assert!(!font.tables.head().unwrap().unwrap().ptr_eq(&head));
/// // to update the font, we need to replace the old table with the modified one:
/// font.tables.insert(head);
/// ```
///
//NOTE:
// This may be too fancy? It is an attempt to make the API more clear, by
// avoiding the need to do `Rc::make_mut(&mut my_table)` every time the user
// wants to mutate a table, but we could just have a
// `Cow::make_mut(&mut self) -> &mut T` method, which would be more explicit?
#[derive(Clone, Debug)]
pub struct CowPtr<T> {
    inner: Rc<T>,
}

impl<T: Clone> CowPtr<T> {
    /// Returns `true` if these two pointers point to the same allocation.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    /// Convert this shared pointer into the inner type, cloning if necessary.
    ///
    /// DerefMut has some limitations; for instance you cannot track field-level
    /// borrows across function barriers. Sometimes it is nice to just deal
    /// with the concrete type.
    pub fn into_owned(self) -> T {
        match Rc::try_unwrap(self.inner) {
            Ok(thing) => thing,
            Err(rc) => T::clone(&rc),
        }
    }
}

impl<T: Clone> std::ops::Deref for CowPtr<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// any access to a mutating method will cause us to ensure we point to unique
// data.
impl<T: Clone> std::ops::DerefMut for CowPtr<T> {
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

    /// Returns `true` if the table store contains no tables.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    ///
    /// [`insert_raw`]: TableSet::insert_raw
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
    pub fn insert_raw(&mut self, tag: Tag, data: impl Into<Rc<[u8]>>) {
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
        let typed_data: LoadedTable = match tag.as_bytes() {
            b"avar" => otspec::de::from_bytes::<tables::avar::avar>(&data)?.into(),
            b"cmap" => otspec::de::from_bytes::<tables::cmap::cmap>(&data)?.into(),
            b"cvt " => otspec::de::from_bytes::<tables::cvt::cvt>(&data)?.into(),
            b"fpgm" => otspec::de::from_bytes::<tables::fpgm::fpgm>(&data)?.into(),
            b"fvar" => otspec::de::from_bytes::<tables::fvar::fvar>(&data)?.into(),
            b"gasp" => otspec::de::from_bytes::<tables::gasp::gasp>(&data)?.into(),
            b"GDEF" => otspec::de::from_bytes::<tables::GDEF::GDEF>(&data)?.into(),
            b"GPOS" => {
                let num_glyphs = self
                    .maxp()?
                    .map(|maxp| maxp.num_glyphs())
                    .ok_or_else(|| DeserializationError("deserialize head before loca".into()))?;
                tables::GPOS::from_bytes(&mut ReaderContext::new(data.to_vec()), num_glyphs)?.into()
            }
            b"GSUB" => {
                let num_glyphs = self
                    .maxp()?
                    .map(|maxp| maxp.num_glyphs())
                    .ok_or_else(|| DeserializationError("deserialize head before loca".into()))?;
                tables::GSUB::from_bytes(&mut ReaderContext::new(data.to_vec()), num_glyphs)?.into()
            }
            b"head" => otspec::de::from_bytes::<tables::head::head>(&data)?.into(),
            b"hhea" => otspec::de::from_bytes::<tables::hhea::hhea>(&data)?.into(),
            b"MATH" => otspec::de::from_bytes::<tables::MATH::MATH>(&data)?.into(),
            b"maxp" => otspec::de::from_bytes::<tables::maxp::maxp>(&data)?.into(),
            b"name" => otspec::de::from_bytes::<tables::name::name>(&data)?.into(),
            b"OS/2" => otspec::de::from_bytes::<tables::os2::os2>(&data)?.into(),
            b"post" => otspec::de::from_bytes::<tables::post::post>(&data)?.into(),
            b"prep" => otspec::de::from_bytes::<tables::prep::prep>(&data)?.into(),
            b"STAT" => otspec::de::from_bytes::<tables::STAT::STAT>(&data)?.into(),
            b"hmtx" => {
                let number_of_hmetrics = self
                    //TODO: dear reviewer: this loads the table if missing. do
                    //we want to force the user to do this explicitly?
                    .hhea()?
                    .map(|hhea| hhea.numberOfHMetrics)
                    //TODO: are we allowed to not have hhea?
                    .ok_or_else(|| DeserializationError("deserialize hhea before hmtx".into()))?;

                tables::hmtx::from_bytes(
                    &mut ReaderContext::new(data.to_vec()),
                    number_of_hmetrics,
                )?
                .into()
            }
            b"loca" => {
                let is_32bit = self
                    .head()?
                    .map(|head| head.indexToLocFormat == 1)
                    .ok_or_else(|| DeserializationError("deserialize head before loca".into()))?;
                tables::loca::from_bytes(&mut ReaderContext::new(data.to_vec()), is_32bit)?.into()
            }
            b"glyf" => {
                let loca = self
                    .loca()?
                    .ok_or_else(|| DeserializationError("deserialize loca before glyf".into()))?;
                tables::glyf::from_bytes(&data, &loca.indices)?.into()
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

                tables::gvar::from_bytes(&data, coords_and_ends)?.into()
            }
            _ => LoadedTable::Unknown(data.clone()),
        };

        Ok(Table {
            raw: Some(data),
            loaded: typed_data,
            tag,
        })
    }

    fn is_serialized(&self, tag: Tag) -> Option<bool> {
        self.tables
            .get(&tag)
            .map(|table| match table.borrow().deref() {
                LazyItem::Unloaded(_) => true,
                LazyItem::Loaded(table) => table.raw.is_some(),
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
                log::warn!("No glyf table");
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
        if glyf_output.is_empty() {
            // Sad special case
            glyf_output.push(0);
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

        let mut maxp = self.maxp().unwrap().unwrap();
        maxp.set_num_glyphs(glyf_count.try_into().unwrap());

        let mut head = self.head().unwrap().unwrap();
        head.indexToLocFormat = if loca_is32bit { 1 } else { 0 };
        self.insert(head);

        if let Some(hmetric_count) = self.hmtx().unwrap().map(|t| t.number_of_hmetrics()) {
            if let Some(mut hhea) = self.hhea().unwrap() {
                hhea.numberOfHMetrics = hmetric_count;
                self.insert(hhea);
            }
        }
    }

    pub(crate) fn compile_gsub_gpos(&mut self) {
        let num_glyphs = self.maxp().unwrap().unwrap().num_glyphs();
        if !self.is_serialized(tables::GPOS::TAG).unwrap_or(true) {
            if let Some(gpos) = self.GPOS().unwrap() {
                let mut gpos_data = vec![];
                if tables::GPOS::to_bytes(&gpos, &mut gpos_data, num_glyphs).is_err() {
                    log::error!("GPOS table overflow");
                }
                self.insert_raw(tables::GPOS::TAG, gpos_data)
            }
        }
        if !self.is_serialized(tables::GSUB::TAG).unwrap_or(true) {
            if let Some(gsub) = self.GSUB().unwrap() {
                let mut gsub_data = vec![];
                if tables::GSUB::to_bytes(&gsub, &mut gsub_data, num_glyphs).is_err() {
                    log::error!("GSUB table overflow");
                }
                self.insert_raw(tables::GSUB::TAG, gsub_data)
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
            LazyItem::Loaded(Table {
                raw: Some(data), ..
            }) => data.to_bytes(buffer),
            LazyItem::Loaded(table) => table.loaded.to_bytes(buffer),
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

impl PartialEq for Table {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.loaded == other.loaded
    }
}

// a little helper to ensure at compile time that tables are Clone;
// we need this for copy-on-write semanatics
fn assert_is_clone<T: Clone>() {}

/// A macro that impls various various conversions and utility methods
/// for typed tables.
macro_rules! table_boilerplate {
    ($table:ty, $enum: ident) => {
        impl From<$table> for LoadedTable {
            fn from(src: $table) -> LoadedTable {
                LoadedTable::$enum(Rc::new(src))
            }
        }

        impl From<$table> for Table {
            fn from(src: $table) -> Table {
                Table::from(CowPtr {
                    inner: Rc::new(src),
                })
            }
        }

        impl From<CowPtr<$table>> for Table {
            fn from(src: CowPtr<$table>) -> Table {
                let loaded = LoadedTable::$enum(src.inner);
                let tag = tables::$enum::TAG;
                Table {
                    raw: None,
                    loaded,
                    tag,
                }
            }
        }

        impl LoadedTable {
            #[allow(non_snake_case)]
            fn $enum(&self) -> CowPtr<$table> {
                assert_is_clone::<$table>();

                if let Self::$enum(table) = self {
                    return CowPtr {
                        inner: table.clone(),
                    };
                } else {
                    panic!("expected '{}' found '{:?}'", tables::$enum::TAG, self);
                }
            }
        }

        impl TableSet {
            #[allow(non_snake_case)]

            /// Get this table, if it exists.
            ///
            /// This returns a pointer which implements copy-on-write semantics.
            /// See the docs for [`CowPtr`] for more details.
            pub fn $enum(&self) -> Result<Option<CowPtr<$table>>, DeserializationError> {
                Ok(self.get(tables::$enum::TAG)?.map(|t| t.loaded.$enum()))
            }
        }
    };
}

table_boilerplate!(tables::GDEF::GDEF, GDEF);
table_boilerplate!(tables::GPOS::GPOS, GPOS);
table_boilerplate!(tables::GSUB::GSUB, GSUB);
table_boilerplate!(tables::STAT::STAT, STAT);
table_boilerplate!(tables::avar::avar, avar);
table_boilerplate!(tables::cmap::cmap, cmap);
table_boilerplate!(tables::cvt::cvt, cvt);
table_boilerplate!(tables::fpgm::fpgm, fpgm);
table_boilerplate!(tables::fvar::fvar, fvar);
table_boilerplate!(tables::gasp::gasp, gasp);
table_boilerplate!(tables::glyf::glyf, glyf);
table_boilerplate!(tables::gvar::gvar, gvar);
table_boilerplate!(tables::head::head, head);
table_boilerplate!(tables::hhea::hhea, hhea);
table_boilerplate!(tables::hmtx::hmtx, hmtx);
table_boilerplate!(tables::loca::loca, loca);
table_boilerplate!(tables::maxp::maxp, maxp);
table_boilerplate!(tables::name::name, name);
table_boilerplate!(tables::os2::os2, os2);
table_boilerplate!(tables::post::post, post);
table_boilerplate!(tables::prep::prep, prep);
table_boilerplate!(tables::MATH::MATH, MATH);

impl Serialize for LoadedTable {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        match self {
            LoadedTable::Unknown(expr) => expr.to_bytes(data),
            LoadedTable::avar(expr) => expr.to_bytes(data),
            LoadedTable::cmap(expr) => expr.to_bytes(data),
            LoadedTable::cvt(expr) => expr.to_bytes(data),
            LoadedTable::fpgm(expr) => expr.to_bytes(data),
            LoadedTable::fvar(expr) => expr.to_bytes(data),
            LoadedTable::gasp(expr) => expr.to_bytes(data),
            LoadedTable::GDEF(expr) => expr.to_bytes(data),
            LoadedTable::GPOS(_) => unimplemented!(),
            LoadedTable::GSUB(_) => unimplemented!(),
            LoadedTable::gvar(_) => unimplemented!(),
            LoadedTable::head(expr) => expr.to_bytes(data),
            LoadedTable::hhea(expr) => expr.to_bytes(data),
            LoadedTable::hmtx(_) => unimplemented!(),
            LoadedTable::glyf(_) => unimplemented!(),
            LoadedTable::loca(_) => unimplemented!(),
            LoadedTable::maxp(expr) => expr.to_bytes(data),
            LoadedTable::MATH(_) => unimplemented!(),
            LoadedTable::name(expr) => expr.to_bytes(data),
            LoadedTable::os2(expr) => expr.to_bytes(data),
            LoadedTable::post(expr) => expr.to_bytes(data),
            LoadedTable::prep(expr) => expr.to_bytes(data),
            LoadedTable::STAT(expr) => expr.to_bytes(data),
        }
    }
}
