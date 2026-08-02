#![allow(missing_docs)]

use std::{fmt, string::String, vec::Vec};

use serde::{
    Deserializer, Serializer,
    de::{self, Visitor},
    ser::{self, Impossible},
};

#[derive(Debug, Eq, PartialEq)]
enum Token {
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug)]
struct ProbeError(String);

impl fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ProbeError {}

impl ser::Error for ProbeError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

struct ProbeSerializer {
    human_readable: bool,
}

macro_rules! unsupported_scalar {
    ($($name:ident($type:ty)),+ $(,)?) => {
        $(
            fn $name(self, _value: $type) -> Result<Self::Ok, Self::Error> {
                Err(ProbeError(stringify!($name).into()))
            }
        )+
    };
}

impl Serializer for ProbeSerializer {
    type Ok = Token;
    type Error = ProbeError;
    type SerializeSeq = Impossible<Token, ProbeError>;
    type SerializeTuple = Impossible<Token, ProbeError>;
    type SerializeTupleStruct = Impossible<Token, ProbeError>;
    type SerializeTupleVariant = Impossible<Token, ProbeError>;
    type SerializeMap = Impossible<Token, ProbeError>;
    type SerializeStruct = Impossible<Token, ProbeError>;
    type SerializeStructVariant = Impossible<Token, ProbeError>;

    unsupported_scalar!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_f32(f32),
        serialize_f64(f64),
        serialize_char(char),
    );

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Token::String(value.into()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Token::Bytes(value.into()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError("serialize_none".into()))
    }

    fn serialize_some<T: ?Sized + serde::Serialize>(
        self,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError("serialize_some".into()))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError("serialize_unit".into()))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError("serialize_unit_struct".into()))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError("serialize_unit_variant".into()))
    }

    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError("serialize_newtype_struct".into()))
    }

    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ProbeError("serialize_newtype_variant".into()))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(ProbeError("serialize_seq".into()))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(ProbeError("serialize_tuple".into()))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ProbeError("serialize_tuple_struct".into()))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ProbeError("serialize_tuple_variant".into()))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ProbeError("serialize_map".into()))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(ProbeError("serialize_struct".into()))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ProbeError("serialize_struct_variant".into()))
    }

    fn is_human_readable(&self) -> bool {
        self.human_readable
    }
}

struct BinaryBytes<'de>(&'de [u8]);

impl<'de> Deserializer<'de> for BinaryBytes<'de> {
    type Error = de::value::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_bytes(self.0)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_bytes(self.0)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_bytes(self.0)
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum
        identifier ignored_any
    }
}

macro_rules! assert_modes {
    ($module:ident, $plain:expr, $encoded:expr) => {{
        let plain: &[u8] = $plain;
        assert_eq!(
            base64_ng_serde::$module::serialize(
                plain,
                ProbeSerializer {
                    human_readable: true
                }
            )
            .unwrap(),
            Token::String($encoded.into())
        );
        assert_eq!(
            base64_ng_serde::$module::serialize(
                plain,
                ProbeSerializer {
                    human_readable: false
                }
            )
            .unwrap(),
            Token::Bytes($encoded.as_bytes().into())
        );
        assert_eq!(
            base64_ng_serde::$module::deserialize(BinaryBytes($encoded.as_bytes())).unwrap(),
            plain
        );
    }};
}

#[test]
fn every_field_profile_has_explicit_human_and_binary_text_modes() {
    assert_modes!(standard, b"hello", "aGVsbG8=");
    assert_modes!(standard_no_pad, b"hello", "aGVsbG8");
    assert_modes!(url_safe, &[0xfb, 0xff], "-_8=");
    assert_modes!(url_safe_no_pad, &[0xfb, 0xff], "-_8");
    assert_modes!(
        mime,
        &[b'a'; 58],
        concat!(
            "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFh\r\n",
            "YQ=="
        )
    );
    assert_modes!(
        pem,
        &[b'a'; 49],
        concat!(
            "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFh\n",
            "YQ=="
        )
    );
}
