use std::fmt;
use std::fmt::{Formatter, LowerExp};

use chrono::NaiveDateTime;
use indoc::writedoc;

use solar_system_info_rpc::solar_system_info::{Planet, Satellite, Type};

pub struct PlanetWrapper<'a>(pub &'a Planet);

impl fmt::Display for PlanetWrapper<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let planet = &self.0;
        let planet_type = convert_planet_type_to_string(
            Type::from_i32(planet.r#type).expect("Planet type not found"),
        );
        let mass = format!("{:e}", PlanetMassWrapper(planet.mass));

        writedoc!(f, "
            <b>Information on {name}</b>

            <b>type:</b> {type}
            <b>mean radius:</b> {mean_radius} km
            <b>mass:</b> {mass} kg
            ",
            name = planet.name,
            type = planet_type,
            mean_radius = planet.mean_radius,
            mass = mass,
        )?;

        if !planet.satellites.is_empty() {
            writedoc!(f, "<b>satellites:</b>\n")?;
            for satellite in &planet.satellites {
                writedoc!(
                    f,
                    "    {satellite}",
                    satellite = SatelliteWrapper(satellite)
                )?;
                writedoc!(f, "\n")?;
            }
        }
        Ok(())
    }
}

struct PlanetMassWrapper(f32);

impl LowerExp for PlanetMassWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        LowerExp::fmt(&self.0, f)
    }
}

struct SatelliteWrapper<'a>(&'a Satellite);

impl fmt::Display for SatelliteWrapper<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let satellite = &self.0;

        writedoc!(f, "<b>{name}</b>", name = satellite.name)?;

        if let Some(timestamp) = &satellite.first_spacecraft_landing_date {
            let date = NaiveDateTime::from_timestamp(timestamp.seconds, 0).date();
            writedoc!(f, " (first spacecraft landing date: {date})", date = date)?;
        }

        Ok(())
    }
}

fn convert_planet_type_to_string<'a>(planet_type: Type) -> &'a str {
    match planet_type {
        Type::TerrestrialPlanet => "Terrestrial planet",
        Type::GasGiant => "Gas giant",
        Type::IceGiant => "Ice giant",
        Type::DwarfPlanet => "Dwarf planet",
    }
}
