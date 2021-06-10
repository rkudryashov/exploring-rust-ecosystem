use std::str::FromStr;

use crate::dto::{PlanetDto, SatelliteDto};
use chrono::Utc;
use mongodb::bson::{self, doc, oid::ObjectId, Document};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct Planet {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub r#type: PlanetType,
    pub mean_radius: f32,
    pub satellites: Option<Vec<Satellite>>,
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub enum PlanetType {
    TerrestrialPlanet,
    GasGiant,
    IceGiant,
    DwarfPlanet,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Satellite {
    pub name: String,
    pub first_spacecraft_landing_date: Option<mongodb::bson::DateTime>,
}

impl From<&Planet> for Document {
    fn from(source: &Planet) -> Self {
        bson::to_document(source).expect("Can't convert a planet to Document")
    }
}

impl From<PlanetDto> for Planet {
    fn from(source: PlanetDto) -> Self {
        Planet {
            id: source
                .id
                .map(|id| ObjectId::from_str(id.as_str()).expect("Can't convert to ObjectId")),
            name: source.name,
            r#type: source.r#type,
            mean_radius: source.mean_radius,
            satellites: source
                .satellites
                .map(|satellites| satellites.into_iter().map(Satellite::from).collect()),
        }
    }
}

impl From<SatelliteDto> for Satellite {
    fn from(source: SatelliteDto) -> Self {
        Satellite {
            name: source.name,
            first_spacecraft_landing_date: source.first_spacecraft_landing_date.map(|d| {
                mongodb::bson::DateTime::from_millis(
                    chrono::Date::<Utc>::from_utc(d, Utc)
                        .and_hms(0, 0, 0)
                        .timestamp_millis(),
                )
            }),
        }
    }
}

impl fmt::Display for PlanetType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
