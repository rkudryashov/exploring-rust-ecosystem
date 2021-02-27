use diesel::prelude::*;

use crate::Conn;
use crate::persistence::model::{PlanetEntity, SatelliteEntity};
use crate::persistence::schema::planets;

pub fn get_names(conn: &Conn) -> QueryResult<Vec<String>> {
    Ok(
        planets::table
            .select(planets::name)
            .load(conn)?
    )
}

pub fn get_all(conn: &Conn) -> QueryResult<Vec<(PlanetEntity, Vec<SatelliteEntity>)>> {
    let planets: Vec<PlanetEntity> = planets::table.load(conn)?;
    let satellites = SatelliteEntity::belonging_to(&planets)
        .load(conn)?
        .grouped_by(&planets);

    let result = planets.into_iter().zip(satellites).collect::<Vec<_>>();

    Ok(result)
}

pub fn get_by_name(name: &str, conn: &Conn) -> QueryResult<(PlanetEntity, Vec<SatelliteEntity>)> {
    let planet: PlanetEntity = planets::table
        .filter(planets::name.ilike(name))
        .first(conn)?;
    let satellites = SatelliteEntity::belonging_to(&planet)
        .load(conn)?;

    Ok((planet, satellites))
}
