#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::pin::Pin;

use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::result::Error;
use diesel::PgConnection;
use dotenv::dotenv;
use futures::Stream;
use log::{debug, error, info};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{transport::Server, Request, Response, Status};

use solar_system_info_rpc::solar_system_info::solar_system_info_server::{
    SolarSystemInfo, SolarSystemInfoServer,
};
use solar_system_info_rpc::solar_system_info::{
    Planet, PlanetRequest, PlanetResponse, PlanetsListResponse,
};

use crate::conversion::PlanetWrapper;
use crate::persistence::connection::{create_connection_pool, PgPool};

embed_migrations!();

mod conversion;
mod persistence;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();

    info!("Starting Solar System info server");

    let addr = std::env::var("GRPC_SERVER_ADDRESS")?.parse()?;

    let pool = create_connection_pool();
    run_migrations(&pool);

    let solar_system_info = SolarSystemInfoService { pool };
    let svc = SolarSystemInfoServer::new(solar_system_info);

    Server::builder().add_service(svc).serve(addr).await?;

    Ok(())
}

struct SolarSystemInfoService {
    pool: PgPool,
}

#[tonic::async_trait]
impl SolarSystemInfo for SolarSystemInfoService {
    type GetPlanetsStream =
        Pin<Box<dyn Stream<Item = Result<PlanetResponse, Status>> + Send + Sync + 'static>>;

    async fn get_planets_list(
        &self,
        request: Request<()>,
    ) -> Result<Response<PlanetsListResponse>, Status> {
        debug!("Got a request: {:?}", request);

        let names_of_planets = persistence::repository::get_names(&get_connection(&self.pool))
            .expect("Can't get names of the planets");

        let reply = PlanetsListResponse {
            list: names_of_planets,
        };

        Ok(Response::new(reply))
    }

    async fn get_planets(
        &self,
        request: Request<()>,
    ) -> Result<Response<Self::GetPlanetsStream>, Status> {
        debug!("Got a request: {:?}", request);

        let (tx, rx) = mpsc::channel(4);

        let planets: Vec<Planet> = persistence::repository::get_all(&get_connection(&self.pool))
            .expect("Can't load planets")
            .into_iter()
            .map(|p| {
                PlanetWrapper {
                    planet: p.0,
                    satellites: p.1,
                }
                .into()
            })
            .collect();

        tokio::spawn(async move {
            let mut stream = tokio_stream::iter(&planets);

            while let Some(planet) = stream.next().await {
                tx.send(Ok(PlanetResponse {
                    planet: Some(planet.clone()),
                }))
                .await
                .unwrap();
            }
        });

        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        )))
    }

    async fn get_planet(
        &self,
        request: Request<PlanetRequest>,
    ) -> Result<Response<PlanetResponse>, Status> {
        debug!("Got a request: {:?}", request);

        let planet_name = request.into_inner().name;

        let planet =
            persistence::repository::get_by_name(&planet_name, &get_connection(&self.pool));

        match planet {
            Ok(planet) => {
                let planet = PlanetWrapper {
                    planet: planet.0,
                    satellites: planet.1,
                }
                .into();

                let reply = PlanetResponse {
                    planet: Some(planet),
                };

                Ok(Response::new(reply))
            }
            Err(e) => {
                error!(
                    "There was an error while getting a planet {}: {}",
                    &planet_name, e
                );
                match e {
                    Error::NotFound => Err(Status::not_found(format!(
                        "Planet with name {} not found",
                        &planet_name
                    ))),
                    _ => Err(Status::unknown(format!(
                        "There was an error while getting a planet {}: {}",
                        &planet_name, e
                    ))),
                }
            }
        }
    }
}

fn run_migrations(pool: &PgPool) {
    let conn = pool.get().expect("Can't get DB connection");
    embedded_migrations::run(&conn).expect("Failed to run database migrations");
}

type Conn = PooledConnection<ConnectionManager<PgConnection>>;

fn get_connection(pool: &PgPool) -> Conn {
    pool.get().expect("Can't get DB connection")
}
