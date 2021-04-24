use bigdecimal::ToPrimitive;
use chrono::NaiveTime;
use rust_embed::RustEmbed;

use solar_system_info_rpc::solar_system_info::{
    Planet as GrpcPlanet, Satellite as GrpcSatellite, Type as GrpcType,
};

use crate::persistence::model::{PlanetEntity, SatelliteEntity};

pub struct PlanetWrapper {
    pub(crate) planet: PlanetEntity,
    pub(crate) satellites: Vec<SatelliteEntity>,
}

impl From<PlanetWrapper> for GrpcPlanet {
    fn from(pw: PlanetWrapper) -> GrpcPlanet {
        let planet = pw.planet;
        let planet_type: GrpcType = convert_string_to_planet_type(&planet.type_);

        let filename = format!("{}.jpg", planet.name.to_lowercase());
        let image = Asset::get(&filename).expect("Failed to open image");

        GrpcPlanet {
            id: planet.id as u64,
            name: planet.name,
            r#type: planet_type.into(),
            mean_radius: planet.mean_radius.to_f32().expect("Can't convert to f32"),
            mass: planet.mass.to_f32().expect("Can't convert mass"),
            satellites: pw.satellites.into_iter().map(|s| s.into()).collect(),
            image: image.to_vec(),
        }
    }
}

impl From<SatelliteEntity> for GrpcSatellite {
    fn from(entity: SatelliteEntity) -> Self {
        let first_spacecraft_landing_date: Option<::prost_types::Timestamp> = entity
            .first_spacecraft_landing_date
            .map(|d| ::prost_types::Timestamp {
                seconds: d.and_time(NaiveTime::from_hms(0, 0, 0)).timestamp(),
                nanos: 0,
            });
        GrpcSatellite {
            id: entity.id as u64,
            name: entity.name,
            first_spacecraft_landing_date,
        }
    }
}

fn convert_string_to_planet_type(planet_type: &str) -> GrpcType {
    match planet_type {
        "TERRESTRIAL_PLANET" => GrpcType::TerrestrialPlanet,
        "GAS_GIANT" => GrpcType::GasGiant,
        "ICE_GIANT" => GrpcType::IceGiant,
        "DWARF_PLANET" => GrpcType::DwarfPlanet,
        _ => panic!("Planet type {} is not found", planet_type),
    }
}

#[derive(RustEmbed)]
#[folder = "images"]
struct Asset;
