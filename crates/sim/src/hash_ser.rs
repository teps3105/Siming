//! GDD 6.5：以 serde 結構把任何 `Serialize` 值依欄位順序餵入雜湊（浮點取位元）。
//! 新增欄位自動納入世界雜湊；`BTreeMap`／`BTreeSet`／`Vec` 的順序固定。

use std::hash::Hasher;

use rustc_hash::FxHasher;
use serde::ser::{self, Serialize};

pub struct HashSer<'a>(pub &'a mut FxHasher);

#[derive(Debug)]
pub struct HashErr(String);
impl std::fmt::Display for HashErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for HashErr {}
impl ser::Error for HashErr {
    fn custom<T: std::fmt::Display>(m: T) -> Self {
        HashErr(m.to_string())
    }
}

/// 把 `v` 餵入 `h`。
pub fn feed<T: Serialize + ?Sized>(h: &mut FxHasher, v: &T) {
    // 本專案型別的序列長度皆已知，不會出錯。
    v.serialize(&mut HashSer(h)).expect("hash serialize");
}

type R = Result<(), HashErr>;

impl<'a, 'b> ser::Serializer for &'a mut HashSer<'b> {
    type Ok = ();
    type Error = HashErr;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> R {
        self.0.write_u8(v as u8);
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> R {
        self.0.write_i8(v);
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> R {
        self.0.write_i16(v);
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> R {
        self.0.write_i32(v);
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> R {
        self.0.write_i64(v);
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> R {
        self.0.write_u8(v);
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> R {
        self.0.write_u16(v);
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> R {
        self.0.write_u32(v);
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> R {
        self.0.write_u64(v);
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> R {
        self.0.write_u32(v.to_bits());
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> R {
        self.0.write_u64(v.to_bits());
        Ok(())
    }
    fn serialize_char(self, v: char) -> R {
        self.0.write_u32(v as u32);
        Ok(())
    }
    fn serialize_str(self, v: &str) -> R {
        self.0.write_usize(v.len());
        self.0.write(v.as_bytes());
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> R {
        self.0.write_usize(v.len());
        self.0.write(v);
        Ok(())
    }
    fn serialize_none(self) -> R {
        self.0.write_u8(0);
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> R {
        self.0.write_u8(1);
        v.serialize(self)
    }
    fn serialize_unit(self) -> R {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> R {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, i: u32, _: &'static str) -> R {
        self.0.write_u32(i);
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _: &'static str, v: &T) -> R {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _: &'static str, i: u32, _: &'static str, v: &T) -> R {
        self.0.write_u32(i);
        v.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self, HashErr> {
        self.0.write_usize(len.ok_or_else(|| HashErr("unsized seq".into()))?);
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self, HashErr> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, HashErr> {
        Ok(self)
    }
    fn serialize_tuple_variant(self, _: &'static str, i: u32, _: &'static str, _: usize) -> Result<Self, HashErr> {
        self.0.write_u32(i);
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self, HashErr> {
        self.0.write_usize(len.ok_or_else(|| HashErr("unsized map".into()))?);
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, HashErr> {
        Ok(self)
    }
    fn serialize_struct_variant(self, _: &'static str, i: u32, _: &'static str, _: usize) -> Result<Self, HashErr> {
        self.0.write_u32(i);
        Ok(self)
    }
}

macro_rules! compound {
    ($($tr:ident :: $m:ident),*) => {$(
        impl<'a, 'b> ser::$tr for &'a mut HashSer<'b> {
            type Ok = ();
            type Error = HashErr;
            fn $m<T: ?Sized + Serialize>(&mut self, v: &T) -> R {
                v.serialize(&mut **self)
            }
            fn end(self) -> R {
                Ok(())
            }
        }
    )*};
}
compound!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field
);

impl<'a, 'b> ser::SerializeMap for &'a mut HashSer<'b> {
    type Ok = ();
    type Error = HashErr;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, k: &T) -> R {
        k.serialize(&mut **self)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, v: &T) -> R {
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

impl<'a, 'b> ser::SerializeStruct for &'a mut HashSer<'b> {
    type Ok = ();
    type Error = HashErr;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &'static str, v: &T) -> R {
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}

impl<'a, 'b> ser::SerializeStructVariant for &'a mut HashSer<'b> {
    type Ok = ();
    type Error = HashErr;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &'static str, v: &T) -> R {
        v.serialize(&mut **self)
    }
    fn end(self) -> R {
        Ok(())
    }
}
