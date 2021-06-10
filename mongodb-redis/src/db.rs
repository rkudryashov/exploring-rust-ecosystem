use mongodb::bson::{doc, oid::ObjectId, Document};
use mongodb::{Client, Collection};
use rust_embed::RustEmbed;
use tokio_stream::StreamExt;

use crate::errors::CustomError;
use crate::errors::CustomError::NotFound;
use crate::model::{Planet, PlanetType};

const DB_NAME: &str = "solar_system_info";
const COLLECTION_NAME: &str = "planets";

#[derive(Clone, Debug)]
pub struct MongoDbClient {
    client: Client,
}

impl MongoDbClient {
    pub async fn new(mongodb_uri: String) -> Self {
        let mongodb_client = Client::with_uri_str(mongodb_uri)
            .await
            .expect("Failed to create MongoDB client");

        MongoDbClient {
            client: mongodb_client,
        }
    }

    pub async fn get_planets(
        &self,
        planet_type: Option<PlanetType>,
    ) -> Result<Vec<Planet>, CustomError> {
        let filter = planet_type.map(|pt| {
            doc! { "type": pt.to_string() }
        });

        let mut planets = self.get_planets_collection().find(filter, None).await?;

        let mut result: Vec<Planet> = Vec::new();
        while let Some(planet) = planets.next().await {
            result.push(planet?);
        }

        Ok(result)
    }

    pub async fn create_planet(&self, planet: Planet) -> Result<Planet, CustomError> {
        let collection = self.get_planets_collection();

        let insert_result = collection.insert_one(planet, None).await?;
        let filter = doc! { "_id": &insert_result.inserted_id };
        collection.find_one(filter, None).await?.ok_or(NotFound {
            message: String::from("Can't find a created planet"),
        })
    }

    pub async fn get_planet(&self, id: ObjectId) -> Result<Planet, CustomError> {
        let collection = self.get_planets_collection();

        let filter = doc! { "_id": &id };
        collection.find_one(filter, None).await?.ok_or(NotFound {
            message: format!("Can't find a planet by id: {}", &id),
        })
    }

    pub async fn update_planet(&self, id: ObjectId, planet: Planet) -> Result<Planet, CustomError> {
        let collection = self.get_planets_collection();

        let query = doc! { "_id": &id };
        let update = doc! { "$set": Document::from(&planet) };
        let _update_result = collection.update_one(query, update, None).await?;

        let filter = doc! { "_id": &id };
        collection.find_one(filter, None).await?.ok_or(NotFound {
            message: format!("Can't find an updated planet by id: {}", &id),
        })
    }

    pub async fn delete_planet(&self, id: ObjectId) -> Result<(), CustomError> {
        let collection = self.get_planets_collection();

        let filter = doc! { "_id": &id };
        collection
            .find_one_and_delete(filter, None)
            .await?
            .ok_or(NotFound {
                message: format!("Can't delete a planet by id: {}", id),
            })?;

        Ok(())
    }

    fn get_planets_collection(&self) -> Collection<Planet> {
        self.client
            .database(DB_NAME)
            .collection::<Planet>(COLLECTION_NAME)
    }
}

pub async fn get_image_of_planet(planet_name: &str) -> Vec<u8> {
    let filename = format!("{}.jpg", planet_name.to_lowercase());
    let image = Asset::get(&filename).expect("Failed to open image");
    image.to_vec()
}

#[derive(RustEmbed)]
#[folder = "images"]
struct Asset;
