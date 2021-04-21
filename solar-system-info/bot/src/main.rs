use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenv::dotenv;
use indoc::formatdoc;
use lazy_static::lazy_static;
use reqwest::Url;
use teloxide::{prelude::*, utils::command::BotCommand};
use teloxide::adaptors::DefaultParseMode;
use teloxide::types::InputFile;
use teloxide::types::ParseMode::Html;

use solar_system_info_rpc::solar_system_info::PlanetRequest;
use solar_system_info_rpc::solar_system_info::solar_system_info_client::SolarSystemInfoClient;

use crate::conversion::PlanetWrapper;
use crate::error::SolarSystemInfoBotError;

mod conversion;
mod error;

lazy_static! {
    static ref GRPC_SERVER_ADDRESS: String = std::env::var("GRPC_SERVER_ADDRESS").expect("Can't read gRPC server address");
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    teloxide::enable_logging!();
    log::info!("Starting Solar System info bot");

    let api_url = std::env::var("TELEGRAM_API_URL").expect("Can't get Telegram API URL");
    let api_url = Url::parse(&api_url).expect("Can't parse Telegram API URL");

    let bot = Bot::from_env()
        .set_api_url(api_url)
        .parse_mode(Html)
        .auto_send();

    // TODO: remove this line after the fix will be issued
    let bot = Arc::new(bot);

    let grpc_client = create_grpc_client().await;

    teloxide::commands_repl(
        bot,
        "solar-system-info-bot",
        move |cx, command| answer(cx, command, grpc_client.clone()),
    ).await;
}

type UpdateCtx = UpdateWithCx<Arc<AutoSend<DefaultParseMode<Bot>>>, Message>;

async fn answer(
    ctx: UpdateCtx,
    command: Command,
    grpc_client: SolarSystemInfoClient<tonic::transport::Channel>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match command {
        Command::Start => {
            let username: String = match ctx.update.from() {
                Some(user) => user.username.clone().unwrap_or_default(),
                None => String::new()
            };

            ctx.answer(get_start_message(&username)).await?;
            ctx.answer(Command::descriptions()).await?;
        }
        Command::Help => {
            ctx.answer(Command::descriptions()).await?;
        }
        Command::List => {
            let response = get_planets_list(grpc_client).await?;
            ctx.answer(response).await?;
        }
        Command::Planets => {
            get_planets(grpc_client, ctx).await?;
        }
        Command::Planet(planet_name) => {
            let response = get_planet(grpc_client, planet_name).await?;
            ctx.answer_photo(response.0).caption(response.1).await?;
        }
    };

    Ok(())
}

fn get_start_message(username: &str) -> String {
    formatdoc!(
            "Hi {username}, this bot can show basic information about planets in the Solar System",
            username = &username
        )
}

async fn get_planets_list(mut grpc_client: SolarSystemInfoClient<tonic::transport::Channel>) -> Result<String, SolarSystemInfoBotError> {
    let response = grpc_client.get_planets_list(tonic::Request::new(()))
        .await;

    return match response {
        Ok(response) => {
            let message = response.into_inner().list.into_iter()
                .map(|planet_name| format!("{} (use <code>/planet {}</code>)", planet_name, planet_name.to_lowercase()))
                .collect::<Vec<String>>()
                .join("\n");
            Ok(message)
        }
        Err(e) => {
            log::error!("There was an error while handling list of planets: {}", e);
            Err(SolarSystemInfoBotError::new("Internal error while handling list of planets"))
        }
    };
}

async fn get_planets(mut grpc_client: SolarSystemInfoClient<tonic::transport::Channel>, ctx: UpdateCtx) -> Result<(), SolarSystemInfoBotError> {
    let response = grpc_client.get_planets(tonic::Request::new(()))
        .await;

    match response {
        Ok(response) => {
            let mut planets_stream = response.into_inner();

            while let Some(response) = planets_stream.message().await.expect("Can't get a planet") {
                match response.planet {
                    Some(planet) => {
                        let message = PlanetWrapper(&planet).to_string();
                        match ctx.answer(message).await {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("Something went wrong while handling planets: {}", e);
                            }
                        };
                    }
                    None => {
                        log::error!("Something went wrong while handling planets");
                        return Err(SolarSystemInfoBotError::new("Something went wrong while handling planets"));
                    }
                }
            }
        }
        Err(e) => {
            log::error!("There was an error while handling planets: {}", e);
            return Err(SolarSystemInfoBotError::new("Internal error while handling planets"));
        }
    };

    Ok(())
}

async fn get_planet(mut grpc_client: SolarSystemInfoClient<tonic::transport::Channel>, planet_name: String) -> Result<(InputFile, String), SolarSystemInfoBotError> {
    let request = tonic::Request::new(PlanetRequest { name: planet_name.to_string() });
    let response = grpc_client.get_planet(request)
        .await;

    return match response {
        Ok(response) => {
            match response.into_inner().planet {
                Some(planet) => {
                    let pw = PlanetWrapper(&planet);
                    let file = InputFile::Memory {
                        file_name: format!("{}.png", &planet_name),
                        data: Cow::from(planet.image.clone()),
                    };

                    Ok((file, pw.to_string()))
                }
                None => {
                    log::error!("Something went wrong while handling a planet {}", &planet_name);
                    Err(SolarSystemInfoBotError::new(format!("Something went wrong while handling a planet {}", planet_name).as_str()))
                }
            }
        }
        Err(status) => {
            log::error!("There was an error while handling a planet: {}", status);
            match status.code() {
                tonic::Code::NotFound => Err(SolarSystemInfoBotError::new(format!("Planet with name {} not found", planet_name).as_str())),
                _ => Err(SolarSystemInfoBotError::new(format!("Internal error while handling a planet {}", planet_name).as_str()))
            }
        }
    };
}

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    Start,
    #[command(description = "shows this message")]
    Help,
    #[command(description = "shows list of planets")]
    List,
    #[command(description = "shows info about all planets")]
    Planets,
    #[command(description = "show info about specified planet (for example, enter <code>/planet neptune</code>)")]
    Planet(String),
}

async fn create_grpc_client() -> SolarSystemInfoClient<tonic::transport::Channel> {
    let channel = tonic::transport::Channel::from_static(&GRPC_SERVER_ADDRESS)
        .connect()
        .await
        .expect("Can't create a channel");

    SolarSystemInfoClient::new(channel)
}
