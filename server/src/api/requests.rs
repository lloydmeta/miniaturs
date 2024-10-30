use std::{fmt, str::FromStr};

use serde::{de, Deserialize, Deserializer};

#[derive(Deserialize)]
pub(crate) struct Signature(pub(crate) String);

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct ImageResizePathParam {
    pub target_width: i32,
    pub target_height: i32,
}

impl<'de> serde::Deserialize<'de> for ImageResizePathParam {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(ImageResizeVisitor)
    }
}
impl serde::Serialize for ImageResizePathParam {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.serialize_str(&format!("{}x{}", self.target_width, self.target_height))
    }
}

struct ImageResizeVisitor;

const IMAGE_RESIZE_PARSE_ERROR: &str = "A string with two numbers and an x in between";

impl<'de> de::Visitor<'de> for ImageResizeVisitor {
    type Value = ImageResizePathParam;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(IMAGE_RESIZE_PARSE_ERROR)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value.parse().map_err(E::custom)
    }
}

impl FromStr for ImageResizePathParam {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<_> = s.split('x').collect();
        if split.len() != 2 {
            anyhow::bail!(IMAGE_RESIZE_PARSE_ERROR)
        } else if let &[width_str, height_str] = split.as_slice() {
            let width: i32 = width_str.parse()?;
            let height: i32 = height_str.parse()?;

            Ok(ImageResizePathParam {
                target_width: width,
                target_height: height,
            })
        } else {
            anyhow::bail!(IMAGE_RESIZE_PARSE_ERROR)
        }
    }
}
