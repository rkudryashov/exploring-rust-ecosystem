use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::model::{Planet, PlanetType, Satellite};

#[derive(Serialize, Deserialize)]
pub struct PlanetDto {
    pub id: Option<String>,
    pub name: String,
    pub r#type: PlanetType,
    pub mean_radius: f32,
    pub satellites: Option<Vec<SatelliteDto>>,
}

#[derive(Serialize, Deserialize)]
pub struct SatelliteDto {
    pub name: String,
    pub first_spacecraft_landing_date: Option<NaiveDate>,
}

#[derive(Serialize)]
pub struct PlanetMessage {
    pub id: String,
    pub name: String,
    pub r#type: PlanetType,
}

impl From<Planet> for PlanetDto {
    fn from(source: Planet) -> Self {
        PlanetDto {
            id: source.id.map(|id| id.to_string()),
            name: source.name,
            r#type: source.r#type,
            mean_radius: source.mean_radius,
            satellites: source
                .satellites
                .map(|satellites| satellites.into_iter().map(SatelliteDto::from).collect()),
        }
    }
}

impl From<Satellite> for SatelliteDto {
    fn from(source: Satellite) -> Self {
        SatelliteDto {
            name: source.name,
            first_spacecraft_landing_date: source
                .first_spacecraft_landing_date
                .map(|d| NaiveDateTime::from_timestamp(d.timestamp_millis() / 1000, 0).date()),
        }
    }
}

impl From<&Planet> for PlanetMessage {
    fn from(source: &Planet) -> Self {
        PlanetMessage {
            id: source
                .id
                .map(|id| id.to_string())
                .expect("Planet.id is not specified"),
            name: source.name.clone(),
            r#type: source.r#type,
        }
    }
}
